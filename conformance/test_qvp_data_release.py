#!/usr/bin/env python
"""Contracts for QVP release profiles, package verification and safe extraction."""

import copy
import importlib.util
import io
import json
import os
import sys
import tarfile
from pathlib import Path
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "scripts/qvp-data-release.py"
spec = importlib.util.spec_from_file_location("qvp_data_release", MODULE)
release = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = release
spec.loader.exec_module(release)


def write(path: Path, data: bytes = b"x") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def hashes(root: Path, names: list[str]) -> dict[str, str]:
    return {name: release.sha256(root / name) for name in names}


def data_tree(root: Path, profile: dict) -> None:
    for index, name in enumerate(release.data_files(), 1):
        write(root / name, f"{index}:{name}\n".encode())
    if profile["version_schema"] is None:
        write(root / "NOTICE.txt", b"")
        version = {
            "name": profile["package"],
            "version": profile["release_version"],
            "format_version": 1,
            "pages": 604,
            "surah_names": 114,
            "files": hashes(root, release.data_files()),
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
                "surah_names": 114,
            },
            "payload_files": hashes(root, payload),
        }
    (root / "VERSION.json").write_text(json.dumps(version))


def expect_error(message: str, function) -> None:
    try:
        function()
    except (TypeError, ValueError) as error:
        assert message in str(error), str(error)
    else:
        raise AssertionError(f"accepted invalid input; expected {message!r}")


def test_profiles_are_disjoint_and_exact():
    default = release.profile("data-v0.4.0")
    assert default["source"] == "hafs-kfgqpc"
    assert default["version"] == "v0.4.0"
    assert default["family"] == "qvp"
    assert default["archive"] == "quran-engine-pages-hafs-kfgqpc.tar.gz"
    assert default["bundle"] == "hafs-kfgqpc.tar.br"
    assert default["legacy_redirect"] is True
    assert release.profile("v0.1.0")["release_version"] == "0.1.0"

    qcf = release.profile("data-hafs-qcf-v1-1405h-v0.1.0")
    assert qcf["source"] == "hafs-qcf-v1-1405h"
    assert qcf["version"] == "v0.1.0"
    assert qcf["family"] == "qvp/hafs-qcf-v1-1405h"
    assert qcf["archive"] == "quran-engine-pages-hafs-qcf-v1-1405h-v0.1.0.tar.gz"
    assert qcf["bundle"] == "hafs-qcf-v1-1405h.tar.br"
    assert qcf["edition"] == "hafs-qcf-v1"
    assert qcf["version_schema_version"] == 7
    assert qcf["legacy_redirect"] is False

    for tag in (
        "v0.2.0",
        "data-v01.2.3",
        "data-0.4.0",
        "data-hafs-qcf-v1-v0.1.0",
        "code-v1.0.0",
    ):
        expect_error("unsupported QVP", lambda tag=tag: release.profile(tag))


def exercise_inventory(tag: str) -> None:
    profile = release.profile(tag)
    with TemporaryDirectory() as directory:
        root = Path(directory) / "source"
        output = Path(directory) / "files.tsv"
        data_tree(root, profile)
        report = release.write_inventory(root, profile, output)
        rows = output.read_text().splitlines()
        assert report["files"] == len(rows) == len(release.ordered_files(root, profile))
        assert rows[0].startswith("001.qvp\t")
        assert rows[603].startswith("604.qvp\t")
        assert rows[604].startswith("001.words.json\t")
        assert any(row.startswith("VERSION.json\t") for row in rows)
        assert rows[-1].startswith(("NOTICE.txt\t", "LICENSE.txt\t"))

        if profile["version_schema"] is not None:
            version_path = root / "VERSION.json"
            version = json.loads(version_path.read_text())
            version["source"]["division_sajdah"]["sajdah_marks"] = 14
            version_path.write_text(json.dumps(version))
            expect_error(
                "semantic evidence differs", lambda: release.verify_tree(root, profile)
            )
            data_tree(root, profile)

            version = json.loads(version_path.read_text())
            version["source"]["source_exclusions"]["typed-owner"] = 4220
            version_path.write_text(json.dumps(version))
            expect_error(
                "semantic evidence differs", lambda: release.verify_tree(root, profile)
            )
            data_tree(root, profile)

            version = json.loads(version_path.read_text())
            version["source"]["optical_calibration_sha256"] = "0" * 64
            version_path.write_text(json.dumps(version))
            expect_error(
                "semantic evidence differs", lambda: release.verify_tree(root, profile)
            )
            data_tree(root, profile)

            version = json.loads(version_path.read_text())
            version["source"]["waqf"]["separate_source_glyphs_by_mark"][
                "waqf_jaiz_wasl_awla"
            ] = 1616
            version_path.write_text(json.dumps(version))
            expect_error(
                "semantic evidence differs", lambda: release.verify_tree(root, profile)
            )
            data_tree(root, profile)

        page = root / "042.qvp"
        original = page.read_bytes()
        page.write_bytes(b"changed")
        expect_error("digest differs", lambda: release.verify_tree(root, profile))
        page.write_bytes(original)

        write(root / "foreign.bin")
        expect_error("inventory differs", lambda: release.verify_tree(root, profile))
        (root / "foreign.bin").unlink()

        sidecar = root / "604.words.json"
        sidecar.unlink()
        expect_error("inventory differs", lambda: release.verify_tree(root, profile))


def test_default_inventory_is_verified():
    exercise_inventory("data-v0.4.0")


def test_qcf_inventory_is_verified():
    exercise_inventory("data-hafs-qcf-v1-1405h-v0.1.0")


def archive(path: Path, package: str, members: list[tuple[str, bytes | None]]) -> None:
    with tarfile.open(path, "w:gz") as output:
        root = tarfile.TarInfo(package)
        root.type = tarfile.DIRTYPE
        output.addfile(root)
        for name, data in members:
            info = tarfile.TarInfo(f"{package}/{name}")
            if data is None:
                info.type = tarfile.SYMTYPE
                info.linkname = "target"
                output.addfile(info)
            else:
                info.size = len(data)
                output.addfile(info, io.BytesIO(data))


def test_bundle_is_metadata_normalized_and_deterministic():
    profile = release.profile("data-hafs-qcf-v1-1405h-v0.1.0")
    with TemporaryDirectory() as directory:
        root = Path(directory)
        first_source = root / "first"
        second_source = root / "second"
        data_tree(first_source, profile)
        data_tree(second_source, profile)
        for index, path in enumerate(sorted(second_source.rglob("*")), 1):
            if path.is_file():
                path.chmod(0o600 if index % 2 else 0o755)
                os.utime(path, (1_500_000_000 + index, 1_500_000_000 + index))

        first_inventory = root / "first.tsv"
        second_inventory = root / "second.tsv"
        release.write_inventory(first_source, profile, first_inventory)
        release.write_inventory(second_source, profile, second_inventory)
        first_bundle = root / "first.tar"
        second_bundle = root / "second.tar"
        first_report = release.write_bundle(
            first_source, profile, first_inventory, first_bundle
        )
        second_report = release.write_bundle(
            second_source, profile, second_inventory, second_bundle
        )
        assert first_bundle.read_bytes() == second_bundle.read_bytes()
        assert first_report["sha256"] == second_report["sha256"]

        with tarfile.open(first_bundle, "r:") as archive_file:
            members = archive_file.getmembers()
        assert [member.name for member in members] == release.ordered_files(
            first_source, profile
        )
        assert all(member.isfile() for member in members)
        assert all(member.mtime == member.uid == member.gid == 0 for member in members)
        assert all(member.uname == member.gname == "" for member in members)
        assert all(member.mode == 0o644 for member in members)

        rows = first_inventory.read_text().splitlines()
        first_inventory.write_text("\n".join(reversed(rows)) + "\n")
        expect_error(
            "inventory order differs",
            lambda: release.write_bundle(
                first_source, profile, first_inventory, root / "wrong-order.tar"
            ),
        )

        release.write_inventory(first_source, profile, first_inventory)
        rows = first_inventory.read_text().splitlines()
        fields = rows[0].split("\t")
        fields[1] = f"+{fields[1]}"
        rows[0] = "\t".join(fields)
        first_inventory.write_text("\n".join(rows) + "\n")
        expect_error(
            "invalid inventory row",
            lambda: release.write_bundle(
                first_source, profile, first_inventory, root / "noncanonical-size.tar"
            ),
        )

        release.write_inventory(first_source, profile, first_inventory)
        rows = first_inventory.read_text().splitlines()
        fields = rows[0].split("\t")
        fields[2] = "0" * 64
        rows[0] = "\t".join(fields)
        first_inventory.write_text("\n".join(rows) + "\n")
        expect_error(
            "inventory differs",
            lambda: release.write_bundle(
                first_source, profile, first_inventory, root / "wrong-digest.tar"
            ),
        )


def test_safe_archive_extraction():
    profile = release.profile("data-hafs-qcf-v1-1405h-v0.1.0")
    with TemporaryDirectory() as directory:
        root = Path(directory)
        good = root / "good.tar.gz"
        archive(good, profile["package"], [("VERSION.json", b"{}"), ("001.qvp", b"x")])
        destination = root / "out"
        report = release.extract_archive(good, profile, destination)
        assert report["files"] == 2
        assert (destination / "001.qvp").read_bytes() == b"x"

        wrong = root / "wrong.tar.gz"
        archive(wrong, "another-package", [("001.qvp", b"x")])
        expect_error(
            "outside",
            lambda: release.extract_archive(wrong, profile, root / "wrong-out"),
        )

        linked = root / "linked.tar.gz"
        archive(linked, profile["package"], [("001.qvp", None)])
        expect_error(
            "link or special",
            lambda: release.extract_archive(linked, profile, root / "linked-out"),
        )

        traversal = root / "traversal.tar.gz"
        with tarfile.open(traversal, "w:gz") as output:
            info = tarfile.TarInfo(f"{profile['package']}/../outside")
            info.size = 1
            output.addfile(info, io.BytesIO(b"x"))
        expect_error(
            "unsafe archive member",
            lambda: release.extract_archive(traversal, profile, root / "traversal-out"),
        )
        assert not (root / "outside").exists()

        duplicate = root / "duplicate.tar.gz"
        archive(
            duplicate,
            profile["package"],
            [("001.qvp", b"first"), ("001.qvp", b"second")],
        )
        expect_error(
            "duplicate archive member",
            lambda: release.extract_archive(duplicate, profile, root / "duplicate-out"),
        )


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
