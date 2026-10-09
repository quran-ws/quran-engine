#!/usr/bin/env python
"""Resolve and verify Quran Engine QVP data-release families."""

import argparse
import hashlib
import io
import json
import re
import shutil
import tarfile
from pathlib import Path, PurePosixPath

SHA256 = re.compile(r"[0-9a-f]{64}")
SEMVER = r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
QCF_TAG = re.compile(rf"data-hafs-qcf-v1-1405h-v({SEMVER})")
DEFAULT_TAG = re.compile(rf"data-v({SEMVER})")
ROOT = Path(__file__).resolve().parents[1]
QCF_CONTRACT_PATH = ROOT / "conformance/qcf-v1-data-package.json"
QCF_CONTRACT = json.loads(QCF_CONTRACT_PATH.read_text())
QCF_SOURCE = QCF_CONTRACT["source"]
QCF_WAQF = QCF_SOURCE["waqf"]
QCF_DIVISION_SAJDAH = QCF_SOURCE["division_sajdah"]
QCF_SOURCE_EXCLUSIONS = QCF_SOURCE["source_exclusions"]
QCF_PATH_GEOMETRY = QCF_SOURCE["base_path_geometry"]
QCF_SOURCE_DIGESTS = {
    name: value
    for name, value in QCF_SOURCE.items()
    if isinstance(value, str) and SHA256.fullmatch(value)
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def profile(tag: str) -> dict:
    match = QCF_TAG.fullmatch(tag)
    if match:
        release_version = match.group(1)
        package = "quran-engine-pages-hafs-qcf-v1-1405h"
        return {
            "source": "hafs-qcf-v1-1405h",
            "tag": tag,
            "version": f"v{release_version}",
            "release_version": release_version,
            "family": "qvp/hafs-qcf-v1-1405h",
            "package": package,
            "archive": f"{package}-v{release_version}.tar.gz",
            "bundle": "hafs-qcf-v1-1405h.tar.br",
            "metadata": ["README.md", "LICENSE.txt"],
            "source_dir": "dist/pages-hafs-qcf-v1-1405h",
            "stage_dir": f"dist/cdn/hafs-qcf-v1-1405h/v{release_version}",
            "version_schema": "quran-engine/edition-data-release",
            "version_schema_version": QCF_CONTRACT["schema_version"],
            "edition": "hafs-qcf-v1",
            "print_year_hijri": 1405,
            "legacy_redirect": False,
        }

    match = DEFAULT_TAG.fullmatch(tag)
    if not match and tag == "v0.1.0":
        match = re.fullmatch(rf"v({SEMVER})", tag)
    if match:
        release_version = match.group(1)
        package = "quran-engine-pages-hafs-kfgqpc"
        return {
            "source": "hafs-kfgqpc",
            "tag": tag,
            "version": f"v{release_version}",
            "release_version": release_version,
            "family": "qvp",
            "package": package,
            "archive": f"{package}.tar.gz",
            "bundle": "hafs-kfgqpc.tar.br",
            "metadata": ["NOTICE.txt"],
            "source_dir": "dist/pages",
            "stage_dir": f"dist/cdn/v{release_version}",
            "version_schema": None,
            "version_schema_version": None,
            "edition": None,
            "print_year_hijri": None,
            "legacy_redirect": True,
        }
    raise ValueError(f"unsupported QVP data release tag: {tag}")


def page_files() -> list[str]:
    return [f"{page:03}.qvp" for page in range(1, 605)]


def sidecar_files() -> list[str]:
    return [f"{page:03}.words.json" for page in range(1, 605)]


def surah_files(kind: str) -> list[str]:
    suffix = "qvp" if kind == "qvp" else "svg"
    return [f"surah-names/{kind}/{surah:03}.{suffix}" for surah in range(1, 115)]


def data_files() -> list[str]:
    return [
        *page_files(),
        *sidecar_files(),
        "atlas.qva",
        "atlas.json",
        *surah_files("qvp"),
        "surah-names/qvp/all.qvp",
        *surah_files("svg"),
        "surah-names/svg/all.svg",
        "surah-names/surah-names.woff2",
        "surah-names/surah-names.css",
        "surah-names/map.json",
    ]


def ordered_files(root: Path, release: dict) -> list[str]:
    names = [
        *page_files(),
        *sidecar_files(),
        "atlas.qva",
        "atlas.json",
        "VERSION.json",
        *surah_files("qvp"),
        "surah-names/qvp/all.qvp",
        *surah_files("svg"),
        "surah-names/svg/all.svg",
        "surah-names/surah-names.woff2",
        "surah-names/surah-names.css",
        "surah-names/map.json",
    ]
    names.extend(release["metadata"])
    return names


def read_json(path: Path) -> object:
    return json.loads(path.read_text())


def digest_map(value: object, field: str) -> dict[str, str]:
    if not isinstance(value, dict) or not value:
        raise ValueError(f"VERSION.json {field} is not a digest map")
    result = {}
    for name, digest in value.items():
        if (
            not isinstance(name, str)
            or not name
            or not isinstance(digest, str)
            or SHA256.fullmatch(digest) is None
        ):
            raise ValueError(f"VERSION.json {field} contains an invalid record")
        result[name] = digest
    return result


def verify_default_version(value: object, release: dict, root: Path) -> None:
    if not isinstance(value, dict):
        raise TypeError("VERSION.json must be an object")
    if (
        value.get("name") != release["package"]
        or value.get("version") != release["release_version"]
        or value.get("format_version") != 1
        or value.get("pages") != 604
        or value.get("surah_names") != 114
    ):
        raise ValueError("default QVP release identity differs")
    declared = digest_map(value.get("files"), "files")
    if set(declared) != set(data_files()):
        raise ValueError("default QVP release digest inventory differs")
    verify_digests(root, declared)


def verify_qcf_source(value: object) -> None:
    if not isinstance(value, dict):
        raise TypeError("QCF V1 source evidence must be an object")
    if value != QCF_SOURCE:
        raise ValueError("QCF V1 source semantic evidence differs")


def verify_qcf_version(value: object, release: dict, root: Path) -> None:
    if not isinstance(value, dict):
        raise TypeError("VERSION.json must be an object")
    qvp = value.get("qvp")
    verify_qcf_source(value.get("source"))
    if (
        value.get("schema") != release["version_schema"]
        or value.get("schema_version") != release["version_schema_version"]
        or value.get("package") != release["package"]
        or value.get("version") != release["release_version"]
        or value.get("edition") != release["edition"]
        or value.get("print_year_hijri") != release["print_year_hijri"]
        or value.get("format_version") != 1
        or not isinstance(qvp, dict)
        or qvp.get("pages") != 604
        or qvp.get("word_sidecars") != 604
        or qvp.get("surah_names") != 114
        or qvp.get("files") != len(data_files())
    ):
        raise ValueError("QCF V1 QVP release identity differs")
    declared = digest_map(value.get("payload_files"), "payload_files")
    actual_payload = set(data_files()) | set(release["metadata"])
    if set(declared) != actual_payload:
        raise ValueError("QCF V1 QVP release digest inventory differs")
    verify_digests(root, declared)


def verify_digests(root: Path, declared: dict[str, str]) -> None:
    for name, expected in declared.items():
        path = root / name
        if not path.is_file() or path.is_symlink():
            raise ValueError(f"declared QVP file is missing: {name}")
        actual = sha256(path)
        if actual != expected:
            raise ValueError(f"QVP file digest differs: {name}")


def verify_tree(root: Path, release: dict) -> list[str]:
    if not root.is_dir():
        raise ValueError(f"QVP source directory does not exist: {root}")
    actual = set()
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError(f"QVP source contains a symlink: {path.relative_to(root)}")
        if path.is_file():
            relative = path.relative_to(root).as_posix()
            if path.stat().st_size <= 0 and relative != "NOTICE.txt":
                raise ValueError(f"QVP source contains an empty file: {relative}")
            actual.add(relative)
        elif not path.is_dir():
            raise ValueError(
                f"QVP source contains a special file: {path.relative_to(root)}"
            )

    expected = set(data_files()) | {"VERSION.json", *release["metadata"]}
    if actual != expected:
        missing = sorted(expected - actual)
        unexpected = sorted(actual - expected)
        raise ValueError(
            "QVP source inventory differs: "
            + json.dumps({"missing": missing, "unexpected": unexpected})
        )

    version = read_json(root / "VERSION.json")
    if release["version_schema"] is None:
        verify_default_version(version, release, root)
    else:
        verify_qcf_version(version, release, root)
    return ordered_files(root, release)


def write_inventory(root: Path, release: dict, output: Path) -> dict:
    names = verify_tree(root, release)
    lines = []
    for name in names:
        path = root / name
        lines.append(f"{name}\t{path.stat().st_size}\t{sha256(path)}\n")
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(output.name + ".tmp")
    if temporary.exists():
        raise ValueError(f"temporary inventory exists: {temporary}")
    try:
        temporary.write_text("".join(lines))
        temporary.replace(output)
    except Exception:
        temporary.unlink(missing_ok=True)
        raise
    return {
        "tag": release["tag"],
        "version": release["version"],
        "family": release["family"],
        "files": len(names),
        "inventory": str(output),
    }


def inventory_rows(path: Path) -> list[tuple[str, int, str]]:
    rows = []
    seen = set()
    for number, line in enumerate(path.read_text().splitlines(), 1):
        fields = line.split("\t")
        if len(fields) != 3:
            raise ValueError(f"invalid inventory row {number}")
        name, size_value, digest = fields
        pure = PurePosixPath(name)
        if (
            not name
            or "\\" in name
            or pure.is_absolute()
            or pure.as_posix() != name
            or any(part in {"", ".", ".."} for part in pure.parts)
            or name in seen
            or SHA256.fullmatch(digest) is None
        ):
            raise ValueError(f"invalid inventory row {number}")
        try:
            size = int(size_value)
        except ValueError as error:
            raise ValueError(f"invalid inventory row {number}") from error
        if size < 0 or str(size) != size_value:
            raise ValueError(f"invalid inventory row {number}")
        seen.add(name)
        rows.append((name, size, digest))
    if not rows:
        raise ValueError("inventory is empty")
    return rows


def write_bundle(root: Path, release: dict, inventory: Path, output: Path) -> dict:
    """Write one metadata-normalized ustar in the verified inventory order."""
    names = verify_tree(root, release)
    rows = inventory_rows(inventory)
    if [name for name, _, _ in rows] != names:
        raise ValueError("bundle inventory order differs")
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(output.name + ".tmp")
    if output.exists() or temporary.exists():
        raise ValueError("bundle output already exists")
    try:
        with tarfile.open(temporary, "w", format=tarfile.USTAR_FORMAT) as archive:
            for name, size, digest in rows:
                data = (root / name).read_bytes()
                if len(data) != size or hashlib.sha256(data).hexdigest() != digest:
                    raise ValueError(f"bundle inventory differs: {name}")
                info = tarfile.TarInfo(name)
                info.size = size
                info.mode = 0o644
                info.mtime = 0
                info.uid = 0
                info.gid = 0
                info.uname = ""
                info.gname = ""
                archive.addfile(info, io.BytesIO(data))
        temporary.replace(output)
    except Exception:
        temporary.unlink(missing_ok=True)
        raise
    return {
        "tag": release["tag"],
        "version": release["version"],
        "family": release["family"],
        "files": len(rows),
        "bundle": str(output),
        "bytes": output.stat().st_size,
        "sha256": sha256(output),
    }


def safe_member(member: tarfile.TarInfo, package: str) -> PurePosixPath | None:
    name = member.name
    parts = name.split("/")
    if (
        "\\" in name
        or name.startswith("/")
        or any(part in {"", ".", ".."} for part in parts)
    ):
        raise ValueError(f"unsafe archive member: {name}")
    path = PurePosixPath(*parts)
    if not path.parts or path.parts[0] != package:
        raise ValueError(f"archive member is outside {package}: {name}")
    if len(path.parts) == 1:
        if not member.isdir():
            raise ValueError("archive root is not a directory")
        return None
    relative = PurePosixPath(*path.parts[1:])
    if member.isdir():
        return relative
    if not member.isfile():
        raise ValueError(f"archive contains a link or special member: {name}")
    return relative


def extract_archive(archive: Path, release: dict, destination: Path) -> dict:
    if not archive.is_file():
        raise ValueError(f"release archive does not exist: {archive}")
    temporary = destination.with_name(destination.name + ".tmp")
    if destination.exists() or temporary.exists():
        raise ValueError("release extraction destination already exists")
    temporary.mkdir(parents=True)
    seen = set()
    try:
        with tarfile.open(archive, "r:*") as source:
            for member in source:
                relative = safe_member(member, release["package"])
                if relative is None:
                    continue
                if relative in seen:
                    raise ValueError(f"duplicate archive member: {relative}")
                seen.add(relative)
                target = temporary.joinpath(*relative.parts)
                if member.isdir():
                    target.mkdir(parents=True, exist_ok=True)
                    continue
                stream = source.extractfile(member)
                if stream is None:
                    raise ValueError(f"cannot read archive member: {member.name}")
                target.parent.mkdir(parents=True, exist_ok=True)
                with stream, target.open("wb") as output:
                    shutil.copyfileobj(stream, output)
        temporary.replace(destination)
    except Exception:
        shutil.rmtree(temporary, ignore_errors=True)
        raise
    return {
        "archive": str(archive),
        "package": release["package"],
        "files": len([path for path in destination.rglob("*") if path.is_file()]),
        "destination": str(destination),
    }


def parser() -> argparse.ArgumentParser:
    command = argparse.ArgumentParser(description=__doc__)
    subcommands = command.add_subparsers(dest="command", required=True)

    get_profile = subcommands.add_parser("profile")
    get_profile.add_argument("tag")

    inventory = subcommands.add_parser("inventory")
    inventory.add_argument("tag")
    inventory.add_argument("source_dir", type=Path)
    inventory.add_argument("output", type=Path)

    extract = subcommands.add_parser("extract")
    extract.add_argument("tag")
    extract.add_argument("archive", type=Path)
    extract.add_argument("destination", type=Path)

    bundle = subcommands.add_parser("bundle")
    bundle.add_argument("tag")
    bundle.add_argument("source_dir", type=Path)
    bundle.add_argument("inventory", type=Path)
    bundle.add_argument("output", type=Path)
    return command


def main() -> None:
    args = parser().parse_args()
    release = profile(args.tag)
    if args.command == "profile":
        result = release
    elif args.command == "inventory":
        result = write_inventory(
            args.source_dir.resolve(), release, args.output.resolve()
        )
    elif args.command == "extract":
        result = extract_archive(
            args.archive.resolve(), release, args.destination.resolve()
        )
    else:
        result = write_bundle(
            args.source_dir.resolve(),
            release,
            args.inventory.resolve(),
            args.output.resolve(),
        )
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
