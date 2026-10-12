#!/usr/bin/env python
"""Contracts for deterministic, edition-aware Quran Engine data packages."""

import argparse
import importlib.util
import json
import sys
import tarfile
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "scripts/package-edition-data.py"
spec = importlib.util.spec_from_file_location("package_edition_data", MODULE)
package_data = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = package_data
spec.loader.exec_module(package_data)


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True))


def fixture(root: Path) -> argparse.Namespace:
    source = root / "source"
    qvp = root / "qvp"
    word_counts = [2, 2, 1]
    for page, count in enumerate(word_counts, 1):
        page_path = source / "pages" / f"{page:03}.svg"
        page_path.parent.mkdir(parents=True, exist_ok=True)
        marker = ' data-source="qpc-resize-hq"' if page == 2 else ""
        page_path.write_text(f'<svg data-page="{page}"{marker}>page-{page}</svg>\n')
        write_json(
            source / "index/by-page" / f"{page:03}.json",
            {
                "page": page,
                "words": [
                    {"word_key": f"1:{page}:{word}", "rasm_uthmani": f"w-{word}"}
                    for word in range(1, count + 1)
                ],
            },
        )
        (qvp / f"{page:03}.qvp").parent.mkdir(parents=True, exist_ok=True)
        (qvp / f"{page:03}.qvp").write_bytes(f"qvp-{page}".encode())
        write_json(
            qvp / f"{page:03}.words.json",
            {
                f"1:{page}:{word}": {
                    "rasm_uthmani": f"u-{word}",
                    "rasm_imlai": f"i-{word}",
                    "qpc": f"q-{word}",
                    "rasm": f"r-{word}",
                    "search": f"s-{word}",
                }
                for word in range(1, count + 1)
            },
        )

    pages = list(range(1, 4))
    page_paths = [source / "pages" / f"{page:03}.svg" for page in pages]
    index_paths = [source / "index/by-page" / f"{page:03}.json" for page in pages]
    summary = {
        "schema": "fixture/hq-word-corpus",
        "schema_version": 9,
        "edition": "fixture-edition",
        "print_year_hijri": 1448,
        "pages": pages,
        "counts": {
            "pages": 3,
            "words": 5,
            "hq_words": 1,
            "fallback_words": 4,
            "pages_with_hq": 1,
        },
        "qualification": "source-qualified",
        "base_summary_sha256": "a" * 64,
        "base_waqf_source_glyphs_sha256": "b" * 64,
        "base_division_sajdah_source_sha256": "c" * 64,
        "source_candidate_tree_sha256": "d" * 64,
        "source_manifest_sha256": "e" * 64,
        "source_records_sha256": "6" * 64,
        "optical_calibration_sha256": "7" * 64,
        "source_exclusions": {"flagged": 4},
        "pages_sha256": package_data.named_digest(page_paths),
        "qualified_geometry_pages_sha256": package_data.named_digest(page_paths),
        "base_header_qualified_geometry_pages_sha256": "f" * 64,
        "base_path_geometry": {
            "qualification": "base-order-preserved",
            "base_paths": 4,
            "emitted_paths": 5,
            "restored_paths": 1,
            "base_pages_sha256": "1" * 64,
            "emitted_pages_sha256": "2" * 64,
        },
        "index_sha256": package_data.named_digest(index_paths),
        "waqf": {
            "qualification": "source-glyph-qualified",
            "text_signs": 2,
            "separate_source_glyphs": 1,
            "fused_source_glyphs": 1,
            "text_signs_by_mark": {"waqf_lazim": 2},
            "separate_source_glyphs_by_mark": {"waqf_lazim": 1},
            "fused_source_glyphs_by_mark": {"waqf_lazim": 1},
        },
        "division_sajdah": {
            "qualification": "source-owned",
            "division_starts": 3,
            "mapped_sajdah_source_glyphs": 1,
            "printed_rubu_al_hizb": 2,
            "restored_sajdah_source_glyphs": 1,
            "sajdah_marks": 2,
            "unprinted_rubu_al_hizb": 1,
        },
        "base_source_manifest_sha256": "4" * 64,
    }
    write_json(source / "summary.json", summary)

    (qvp / "atlas.qva").write_bytes(b"atlas")
    write_json(qvp / "atlas.json", {"pages": pages, "surahs": [1, 2]})
    for surah in range(1, 3):
        path = qvp / "surah-names/qvp" / f"{surah:03}.qvp"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(f"surah-qvp-{surah}".encode())
        path = qvp / "surah-names/svg" / f"{surah:03}.svg"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"<svg>{surah}</svg>\n")
    (qvp / "surah-names/qvp/all.qvp").write_bytes(b"all-qvp")
    (qvp / "surah-names/svg/all.svg").write_text("<svg>all</svg>\n")
    (qvp / "surah-names/surah-names.woff2").write_bytes(b"font")
    (qvp / "surah-names/surah-names.css").write_text("font{}\n")
    write_json(qvp / "surah-names/map.json", {"1": 1, "2": 2})

    converter = root / "qvp-convert"
    converter.write_bytes(b"converter")
    licence = root / "LICENSE"
    licence.write_text("fixture licence\n")
    qvp_files = package_data.expected_qvp_files(qvp, 3, 2)
    contract = {
        "schema": "quran-engine/edition-data-package-contract",
        "schema_version": 7,
        "package_name": "fixture-pages",
        "edition": "fixture-edition",
        "print_year_hijri": 1448,
        "format_version": 1,
        "source": {
            "summary_sha256": package_data.sha256(source / "summary.json"),
            "summary_schema": summary["schema"],
            "summary_schema_version": summary["schema_version"],
            "pages_sha256": summary["pages_sha256"],
            "qualified_geometry_pages_sha256": summary[
                "qualified_geometry_pages_sha256"
            ],
            "base_header_qualified_geometry_pages_sha256": summary[
                "base_header_qualified_geometry_pages_sha256"
            ],
            "base_path_geometry": summary["base_path_geometry"],
            "index_sha256": summary["index_sha256"],
            "pages": 3,
            "words": 5,
            "qualification": "source-qualified",
            "hq_words": 1,
            "fallback_words": 4,
            "pages_with_hq": 1,
            "base_summary_sha256": summary["base_summary_sha256"],
            "base_waqf_source_glyphs_sha256": summary["base_waqf_source_glyphs_sha256"],
            "base_division_sajdah_source_sha256": summary[
                "base_division_sajdah_source_sha256"
            ],
            "division_sajdah": summary["division_sajdah"],
            "source_candidate_tree_sha256": summary["source_candidate_tree_sha256"],
            "source_manifest_sha256": summary["source_manifest_sha256"],
            "source_records_sha256": summary["source_records_sha256"],
            "optical_calibration_sha256": summary["optical_calibration_sha256"],
            "source_exclusions": summary["source_exclusions"],
            "waqf": summary["waqf"],
        },
        "qvp": {
            "tree_sha256": package_data.tree_digest(qvp, qvp_files),
            "files": len(qvp_files),
            "pages": 3,
            "word_sidecars": 3,
            "logical_words": 5,
            "atlas_qva_sha256": package_data.sha256(qvp / "atlas.qva"),
            "atlas_json_sha256": package_data.sha256(qvp / "atlas.json"),
            "surah_names": 2,
        },
        "converter": {"sha256": package_data.sha256(converter)},
    }
    contract_path = root / "contract.json"
    write_json(contract_path, contract)
    return argparse.Namespace(
        version="0.1.0",
        source_dir=source,
        qvp_dir=qvp,
        converter=converter,
        contract=contract_path,
        license=licence,
        out_dir=root / "out",
    )


def build(args: argparse.Namespace) -> dict:
    with redirect_stdout(StringIO()):
        return package_data.package(args)


def archive_path(args: argparse.Namespace) -> Path:
    return args.out_dir / "fixture-pages-v0.1.0.tar.gz"


def test_archive_is_byte_deterministic_and_self_describing():
    with TemporaryDirectory() as directory:
        root = Path(directory)
        first = fixture(root / "first")
        second = fixture(root / "second")
        report_a = build(first)
        report_b = build(second)
        archive_a = archive_path(first)
        archive_b = archive_path(second)
        assert archive_a.read_bytes() == archive_b.read_bytes()
        assert report_a["archive_sha256"] == report_b["archive_sha256"]
        assert (first.out_dir / f"{archive_a.name}.sha256").read_text() == (
            second.out_dir / f"{archive_b.name}.sha256"
        ).read_text()

        with tarfile.open(archive_a, "r:gz") as archive:
            members = archive.getmembers()
            names = [member.name for member in members]
            assert names == sorted(names)
            assert len(members) == 20
            assert all(name.startswith("fixture-pages/") for name in names)
            assert all(member.isfile() for member in members)
            assert all(member.mtime == 0 for member in members)
            assert all(member.uid == member.gid == 0 for member in members)
            assert all(member.uname == member.gname == "" for member in members)
            assert all(member.mode == 0o644 for member in members)
            version = json.load(archive.extractfile("fixture-pages/VERSION.json"))
            assert version["schema_version"] == 7
            assert version["edition"] == "fixture-edition"
            assert version["print_year_hijri"] == 1448
            assert "distribution" not in version
            assert version["source_corpus_manifest_sha256"] == "e" * 64
            assert version["base_source_manifest_sha256"] == "4" * 64
            assert "generated_at" not in version
            for relative, expected in version["payload_files"].items():
                value = archive.extractfile(f"fixture-pages/{relative}").read()
                assert package_data.hashlib.sha256(value).hexdigest() == expected
            readme = archive.extractfile("fixture-pages/README.md").read().decode()
            assert "1 qualified HQ word" in readme
            assert "4 verified QCF/KFGQPC fallback words" in readme
            assert "1 independently owned waqf sign" in readme
            assert "1 fused sign" in readme
            assert "3 division boundaries" in readme
            assert "2 printed rosettes" in readme
            assert "1 metadata-only boundary" in readme
            assert "2 sajdah marks" in readme


def test_exact_qvp_inventory_and_digests_are_required():
    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        (args.qvp_dir / "extra.txt").write_text("extra\n")
        try:
            build(args)
        except ValueError as error:
            assert "file set differs" in str(error)
        else:
            raise AssertionError("accepted an extra QVP payload file")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        (args.qvp_dir / "002.qvp").write_bytes(b"changed")
        try:
            build(args)
        except ValueError as error:
            assert "tree digest differs" in str(error)
        else:
            raise AssertionError("accepted changed QVP bytes")


def test_source_identity_converter_and_output_drift_fail_closed():
    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        args.converter.write_bytes(b"changed")
        try:
            build(args)
        except ValueError as error:
            assert "converter digest differs" in str(error)
        else:
            raise AssertionError("accepted another converter")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        summary_path = args.source_dir / "summary.json"
        summary = json.loads(summary_path.read_text())
        summary["counts"]["hq_words"] = 2
        write_json(summary_path, summary)
        contract = json.loads(args.contract.read_text())
        contract["source"]["summary_sha256"] = package_data.sha256(summary_path)
        write_json(args.contract, contract)
        try:
            build(args)
        except ValueError as error:
            assert "source corpus identity differs" in str(error)
        else:
            raise AssertionError("accepted another HQ source identity")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        summary_path = args.source_dir / "summary.json"
        summary = json.loads(summary_path.read_text())
        summary["waqf"]["fused_source_glyphs"] = 2
        write_json(summary_path, summary)
        contract = json.loads(args.contract.read_text())
        contract["source"]["summary_sha256"] = package_data.sha256(summary_path)
        write_json(args.contract, contract)
        try:
            build(args)
        except ValueError as error:
            assert "source corpus identity differs" in str(error)
        else:
            raise AssertionError("accepted changed waqf source identity")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        summary_path = args.source_dir / "summary.json"
        summary = json.loads(summary_path.read_text())
        summary["division_sajdah"]["sajdah_marks"] = 3
        write_json(summary_path, summary)
        contract = json.loads(args.contract.read_text())
        contract["source"]["summary_sha256"] = package_data.sha256(summary_path)
        write_json(args.contract, contract)
        try:
            build(args)
        except ValueError as error:
            assert "source corpus identity differs" in str(error)
        else:
            raise AssertionError("accepted changed division/sajdah source identity")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        summary_path = args.source_dir / "summary.json"
        summary = json.loads(summary_path.read_text())
        summary["optical_calibration_sha256"] = "8" * 64
        write_json(summary_path, summary)
        contract = json.loads(args.contract.read_text())
        contract["source"]["summary_sha256"] = package_data.sha256(summary_path)
        write_json(args.contract, contract)
        try:
            build(args)
        except ValueError as error:
            assert "source corpus identity differs" in str(error)
        else:
            raise AssertionError("accepted changed optical calibration identity")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        build(args)
        try:
            build(args)
        except ValueError as error:
            assert "output already exists" in str(error)
        else:
            raise AssertionError("overwrote an edition archive")


def test_version_and_contract_shape_are_strict():
    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        args.version = "latest"
        try:
            build(args)
        except ValueError as error:
            assert "version must be" in str(error)
        else:
            raise AssertionError("accepted an unversioned package")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        contract = json.loads(args.contract.read_text())
        contract["source"]["waqf"]["text_signs"] = 3
        write_json(args.contract, contract)
        try:
            build(args)
        except ValueError as error:
            assert "waqf counts differ" in str(error)
        else:
            raise AssertionError("accepted an inconsistent waqf contract")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        contract = json.loads(args.contract.read_text())
        waqf = contract["source"]["waqf"]
        waqf["text_signs_by_mark"] = {"waqf_invented": 2}
        waqf["separate_source_glyphs_by_mark"] = {"waqf_invented": 1}
        waqf["fused_source_glyphs_by_mark"] = {"waqf_invented": 1}
        write_json(args.contract, contract)
        try:
            build(args)
        except ValueError as error:
            assert "text_signs_by_mark differs" in str(error)
        else:
            raise AssertionError("accepted an invented waqf mark")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        contract = json.loads(args.contract.read_text())
        contract["source"]["source_exclusions"]["flagged"] = 3
        write_json(args.contract, contract)
        try:
            build(args)
        except ValueError as error:
            assert "source exclusions differ" in str(error)
        else:
            raise AssertionError("accepted incomplete source exclusion accounting")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        contract = json.loads(args.contract.read_text())
        contract["source"]["base_path_geometry"]["emitted_paths"] = 4
        write_json(args.contract, contract)
        try:
            build(args)
        except ValueError as error:
            assert "path-geometry identity differs" in str(error)
        else:
            raise AssertionError("accepted inconsistent path geometry")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        contract = json.loads(args.contract.read_text())
        contract["source"]["division_sajdah"]["unprinted_rubu_al_hizb"] = 2
        write_json(args.contract, contract)
        try:
            build(args)
        except ValueError as error:
            assert "division/sajdah counts differ" in str(error)
        else:
            raise AssertionError("accepted inconsistent division/sajdah counts")

    with TemporaryDirectory() as directory:
        args = fixture(Path(directory))
        contract = json.loads(args.contract.read_text())
        contract["distribution"] = {"status": "public"}
        write_json(args.contract, contract)
        try:
            build(args)
        except ValueError as error:
            assert "contract fields differ" in str(error)
        else:
            raise AssertionError("accepted an obsolete distribution contract")


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
