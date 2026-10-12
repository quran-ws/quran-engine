#!/usr/bin/env python
"""Contracts for pinned, verified and atomic QVP data installation."""

import copy
import hashlib
import importlib.util
import io
import json
import sys
import tarfile
from pathlib import Path
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "scripts/install-qvp-data.py"
spec = importlib.util.spec_from_file_location("install_qvp_data", MODULE)
installer = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = installer
spec.loader.exec_module(installer)
release = installer.release
REGISTRY = ROOT / "conformance/qvp-data-editions.json"


def write(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def data_tree(root: Path, profile: dict) -> None:
    for index, name in enumerate(release.data_files(), 1):
        write(root / name, f"{index}:{name}\n".encode())
    if profile["version_schema"] is None:
        write(root / "NOTICE.txt", b"default notice\n")
        version = {
            "name": profile["package"],
            "version": profile["release_version"],
            "format_version": 1,
            "pages": 604,
            "surah_names": 114,
            "files": {name: digest(root / name) for name in release.data_files()},
        }
    else:
        write(root / "README.md", b"QCF V1 data\n")
        write(root / "LICENSE.txt", b"licence\n")
        payload = [*release.data_files(), "README.md", "LICENSE.txt"]
        version = {
            "schema": profile["version_schema"],
            "schema_version": profile["version_schema_version"],
            "package": profile["package"],
            "version": profile["release_version"],
            "edition": profile["edition"],
            "print_year_hijri": profile["print_year_hijri"],
            "format_version": 1,
            "source": copy.deepcopy(release.QCF_SOURCE),
            "qvp": {
                "files": len(release.data_files()),
                "pages": 604,
                "word_sidecars": 604,
                "logical_words": 77432,
                "surah_names": 114,
            },
            "payload_files": {name: digest(root / name) for name in payload},
        }
    (root / "VERSION.json").write_text(json.dumps(version, sort_keys=True))


def make_archive(source: Path, profile: dict, path: Path) -> None:
    with tarfile.open(path, "w:gz") as output:
        root = tarfile.TarInfo(profile["package"])
        root.type = tarfile.DIRTYPE
        output.addfile(root)
        for source_path in sorted(item for item in source.rglob("*") if item.is_file()):
            relative = source_path.relative_to(source).as_posix()
            data = source_path.read_bytes()
            info = tarfile.TarInfo(f"{profile['package']}/{relative}")
            info.size = len(data)
            output.addfile(info, io.BytesIO(data))


def registry(
    path: Path, source: str, tag: str, archive_sha256: str, published: bool = False
) -> Path:
    path.write_text(
        json.dumps(
            {
                "schema": "quran-engine/qvp-data-editions",
                "schema_version": 1,
                "repository": "quran-ws/quran-engine",
                "default": source,
                "editions": {
                    source: {
                        "tag": tag,
                        "archive_sha256": archive_sha256,
                        "published": published,
                    }
                },
            }
        )
    )
    return path


def expect_error(message: str, function) -> None:
    try:
        function()
    except (TypeError, ValueError) as error:
        assert message in str(error), str(error)
    else:
        raise AssertionError(f"accepted invalid input; expected {message!r}")


def test_registry_matches_current_release_and_consumer_sources():
    default = installer.resolve(REGISTRY, None)
    assert default["source"] == "hafs-kfgqpc"
    assert default["published"] is True
    assert default["tag"] == "data-v0.4.0"
    assert default["family"] == "qvp"
    assert default["archive_url"].endswith(
        "/data-v0.4.0/quran-engine-pages-hafs-kfgqpc.tar.gz"
    )

    qcf = installer.resolve(REGISTRY, "hafs-qcf-v1-1405h")
    assert qcf["published"] is False
    assert qcf["archive_url"] is None
    assert qcf["tag"] == "data-hafs-qcf-v1-1405h-v0.1.0"
    assert qcf["family"] == "qvp/hafs-qcf-v1-1405h"
    assert qcf["archive_sha256"] == (
        "7ec6052d35175c1be893484e88004ce754b2fdffc2c752c3c64632c63036d9f1"
    )


def test_install_is_verified_atomic_and_reverifiable():
    with TemporaryDirectory() as directory:
        root = Path(directory)
        profile = release.profile("data-hafs-qcf-v1-1405h-v0.1.0")
        source = root / "source"
        archive = root / "edition.tar.gz"
        data_tree(source, profile)
        make_archive(source, profile, archive)
        registry_path = registry(
            root / "registry.json",
            profile["source"],
            profile["tag"],
            digest(archive),
        )
        edition = installer.resolve(registry_path, profile["source"])
        assert installer.verify_archive(archive, edition)["files"] == 1446

        destination = root / "installed"
        installed = installer.install_archive(archive, edition, destination)
        assert installed["root"] == str(destination.resolve())
        assert installed["files"] == 1446
        assert installer.verify_installed(destination, edition) == installed
        assert not list(root.glob(f".{destination.name}.*"))

        expect_error(
            "already exists",
            lambda: installer.install_archive(archive, edition, destination),
        )
        (destination / "001.qvp").write_bytes(b"changed")
        expect_error(
            "digest differs",
            lambda: installer.verify_installed(destination, edition),
        )


def test_outer_and_inner_drift_fail_without_installing():
    with TemporaryDirectory() as directory:
        root = Path(directory)
        profile = release.profile("data-hafs-qcf-v1-1405h-v0.1.0")
        source = root / "source"
        archive = root / "edition.tar.gz"
        data_tree(source, profile)
        make_archive(source, profile, archive)

        wrong_registry = registry(
            root / "wrong-registry.json",
            profile["source"],
            profile["tag"],
            "0" * 64,
        )
        wrong = installer.resolve(wrong_registry, profile["source"])
        destination = root / "wrong-install"
        expect_error(
            "archive digest differs",
            lambda: installer.install_archive(archive, wrong, destination),
        )
        assert not destination.exists()

        (source / "001.qvp").write_bytes(b"internally changed")
        make_archive(source, profile, archive)
        drift_registry = registry(
            root / "drift-registry.json",
            profile["source"],
            profile["tag"],
            digest(archive),
        )
        drift = installer.resolve(drift_registry, profile["source"])
        destination = root / "drift-install"
        expect_error(
            "file digest differs",
            lambda: installer.install_archive(archive, drift, destination),
        )
        assert not destination.exists()
        assert not list(root.glob(f".{destination.name}.*"))


def test_stale_registry_identity_and_unpublished_fetch_fail_closed():
    with TemporaryDirectory() as directory:
        root = Path(directory)
        stale = registry(
            root / "stale.json",
            "hafs-qcf-v1-1405h",
            "data-v0.4.0",
            "0" * 64,
        )
        expect_error(
            "edition/tag identity differs",
            lambda: installer.load_registry(stale),
        )

        qcf = installer.resolve(REGISTRY, "hafs-qcf-v1-1405h")
        destination = root / "downloaded"
        expect_error(
            "qualified but not published",
            lambda: installer.fetch(qcf, destination),
        )
        assert not destination.exists()
        assert not list(root.glob(".qvp-data-*.tar.gz"))


if __name__ == "__main__":
    tests = sorted(
        (name, function)
        for name, function in globals().items()
        if name.startswith("test_") and callable(function)
    )
    for name, test in tests:
        test()
        print("ok ", name)
    print(f"\n{len(tests)} tests passed")
