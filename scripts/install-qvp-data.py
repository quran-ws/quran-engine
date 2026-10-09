#!/usr/bin/env python
"""Verify, install, or fetch one pinned QVP data edition."""

import argparse
import hashlib
import importlib.util
import json
import os
import re
import shutil
import sys
import tempfile
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_REGISTRY = ROOT / "conformance/qvp-data-editions.json"
RELEASE_TOOL = ROOT / "scripts/qvp-data-release.py"
SHA256 = re.compile(r"[0-9a-f]{64}")
IDENTIFIER = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")
REPOSITORY = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")
MAX_ARCHIVE_BYTES = 1_000_000_000


def load_release_tool():
    spec = importlib.util.spec_from_file_location("qvp_data_release", RELEASE_TOOL)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


release = load_release_tool()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def load_registry(path: Path) -> dict:
    value = json.loads(path.read_text())
    if not isinstance(value, dict) or set(value) != {
        "schema",
        "schema_version",
        "repository",
        "default",
        "editions",
    }:
        raise ValueError("QVP edition registry fields differ")
    if (
        value["schema"] != "quran-engine/qvp-data-editions"
        or value["schema_version"] != 1
        or not isinstance(value["repository"], str)
        or REPOSITORY.fullmatch(value["repository"]) is None
        or not isinstance(value["editions"], dict)
        or not value["editions"]
        or value["default"] not in value["editions"]
    ):
        raise ValueError("QVP edition registry identity differs")

    for identifier, entry in value["editions"].items():
        if (
            not isinstance(identifier, str)
            or IDENTIFIER.fullmatch(identifier) is None
            or not isinstance(entry, dict)
            or set(entry) != {"tag", "archive_sha256", "published"}
            or not isinstance(entry["tag"], str)
            or not isinstance(entry["archive_sha256"], str)
            or SHA256.fullmatch(entry["archive_sha256"]) is None
            or not isinstance(entry["published"], bool)
        ):
            raise ValueError(f"invalid QVP edition registry entry: {identifier!r}")
        profile = release.profile(entry["tag"])
        if profile["source"] != identifier:
            raise ValueError(f"QVP edition/tag identity differs: {identifier}")
    return value


def resolve(registry_path: Path, identifier: str | None) -> dict:
    registry = load_registry(registry_path)
    identifier = identifier or registry["default"]
    entry = registry["editions"].get(identifier)
    if entry is None:
        raise ValueError(f"unknown QVP data edition: {identifier}")
    profile = release.profile(entry["tag"])
    url = None
    if entry["published"]:
        url = (
            f"https://github.com/{registry['repository']}/releases/download/"
            f"{entry['tag']}/{profile['archive']}"
        )
    return {
        **profile,
        "archive_sha256": entry["archive_sha256"],
        "published": entry["published"],
        "archive_url": url,
        "registry_sha256": sha256(registry_path),
    }


def check_archive(archive: Path, edition: dict) -> None:
    if not archive.is_file() or archive.is_symlink():
        raise ValueError(f"QVP archive is missing or unsafe: {archive}")
    actual = sha256(archive)
    if actual != edition["archive_sha256"]:
        raise ValueError(
            f"QVP archive digest differs: expected {edition['archive_sha256']}, got {actual}"
        )


def extract_verified(archive: Path, edition: dict, destination: Path) -> list[str]:
    check_archive(archive, edition)
    release.extract_archive(archive, edition, destination)
    try:
        return release.verify_tree(destination, edition)
    except Exception:
        shutil.rmtree(destination, ignore_errors=True)
        raise


def report(edition: dict, root: Path, names: list[str]) -> dict:
    return {
        "source": edition["source"],
        "tag": edition["tag"],
        "version": edition["version"],
        "edition": edition["edition"],
        "print_year_hijri": edition["print_year_hijri"],
        "package": edition["package"],
        "family": edition["family"],
        "archive_sha256": edition["archive_sha256"],
        "version_sha256": sha256(root / "VERSION.json"),
        "files": len(names),
        "root": str(root),
    }


def verify_archive(archive: Path, edition: dict) -> dict:
    with tempfile.TemporaryDirectory(prefix="qvp-data-verify-") as directory:
        root = Path(directory) / "data"
        names = extract_verified(archive, edition, root)
        result = report(edition, root, names)
    result.pop("root")
    return result


def verify_installed(root: Path, edition: dict) -> dict:
    names = release.verify_tree(root, edition)
    return report(edition, root, names)


def install_archive(archive: Path, edition: dict, destination: Path) -> dict:
    if destination.exists():
        raise ValueError(f"installation destination already exists: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(
        tempfile.mkdtemp(prefix=f".{destination.name}.", dir=destination.parent)
    )
    payload = staging / "data"
    try:
        names = extract_verified(archive, edition, payload)
        os.replace(payload, destination)
    except Exception:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    shutil.rmtree(staging, ignore_errors=True)
    return report(edition, destination, names)


def download(edition: dict, destination: Path) -> None:
    if not edition["published"]:
        raise ValueError(
            f"QVP edition {edition['source']} is qualified but not published; "
            "install its local checksummed archive instead"
        )
    request = urllib.request.Request(
        edition["archive_url"], headers={"User-Agent": "quran-engine-data-installer/1"}
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            length = response.headers.get("Content-Length")
            if length is not None and int(length) > MAX_ARCHIVE_BYTES:
                raise ValueError("QVP archive exceeds the download size limit")
            total = 0
            with destination.open("wb") as output:
                while chunk := response.read(1024 * 1024):
                    total += len(chunk)
                    if total > MAX_ARCHIVE_BYTES:
                        raise ValueError("QVP archive exceeds the download size limit")
                    output.write(chunk)
    except (OSError, urllib.error.URLError) as error:
        destination.unlink(missing_ok=True)
        raise ValueError(f"could not download QVP archive: {error}") from error


def fetch(edition: dict, destination: Path) -> dict:
    destination.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(
        prefix=".qvp-data-", suffix=".tar.gz", dir=destination.parent
    )
    os.close(descriptor)
    temporary = Path(name)
    try:
        download(edition, temporary)
        return install_archive(temporary, edition, destination)
    finally:
        temporary.unlink(missing_ok=True)


def parser() -> argparse.ArgumentParser:
    command = argparse.ArgumentParser(description=__doc__)
    command.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY)
    subcommands = command.add_subparsers(dest="command", required=True)

    resolve_command = subcommands.add_parser("resolve")
    resolve_command.add_argument("source", nargs="?")

    verify_archive_command = subcommands.add_parser("verify-archive")
    verify_archive_command.add_argument("source")
    verify_archive_command.add_argument("archive", type=Path)

    verify_installed_command = subcommands.add_parser("verify-installed")
    verify_installed_command.add_argument("source")
    verify_installed_command.add_argument("directory", type=Path)

    install_command = subcommands.add_parser("install")
    install_command.add_argument("source")
    install_command.add_argument("archive", type=Path)
    install_command.add_argument("destination", type=Path)

    fetch_command = subcommands.add_parser("fetch")
    fetch_command.add_argument("source", nargs="?")
    fetch_command.add_argument("destination", type=Path)
    return command


def main() -> None:
    args = parser().parse_args()
    registry = args.registry.resolve()
    edition = resolve(registry, getattr(args, "source", None))
    if args.command == "resolve":
        result = edition
    elif args.command == "verify-archive":
        result = verify_archive(args.archive.resolve(), edition)
    elif args.command == "verify-installed":
        result = verify_installed(args.directory.resolve(), edition)
    elif args.command == "install":
        result = install_archive(
            args.archive.resolve(), edition, args.destination.resolve()
        )
    else:
        result = fetch(edition, args.destination.resolve())
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
