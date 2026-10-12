#!/usr/bin/env python
"""Build a deterministic, contract-pinned Quran Engine edition data archive."""

import argparse
import gzip
import hashlib
import io
import json
import re
import shutil
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONTRACT = ROOT / "conformance/qcf-v1-data-package.json"
VERSION = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
WAQF_MARKS = {
    "waqf_al_muanaqah",
    "waqf_jaiz_mustawi_al_tarafayn",
    "waqf_jaiz_waqf_awla",
    "waqf_jaiz_wasl_awla",
    "waqf_lazim",
}
PATH_GEOMETRY_FIELDS = {
    "qualification",
    "base_paths",
    "emitted_paths",
    "restored_paths",
    "base_pages_sha256",
    "emitted_pages_sha256",
}
DIVISION_SAJDAH_FIELDS = {
    "qualification",
    "division_starts",
    "mapped_sajdah_source_glyphs",
    "printed_rubu_al_hizb",
    "restored_sajdah_source_glyphs",
    "sajdah_marks",
    "unprinted_rubu_al_hizb",
}


def read_json(path: Path) -> object:
    return json.loads(path.read_text())


def json_bytes(value: object) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    ).encode()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def named_digest(paths: list[Path]) -> str:
    digest = hashlib.sha256()
    for path in paths:
        digest.update(path.name.encode())
        digest.update(b"\0")
        with path.open("rb") as source:
            while chunk := source.read(1024 * 1024):
                digest.update(chunk)
    return digest.hexdigest()


def tree_digest(root: Path, files: list[Path]) -> str:
    digest = hashlib.sha256()
    for path in files:
        digest.update(path.relative_to(root).as_posix().encode())
        digest.update(b"\0")
        with path.open("rb") as source:
            while chunk := source.read(1024 * 1024):
                digest.update(chunk)
    return digest.hexdigest()


def validate_waqf(value: object) -> dict:
    fields = {
        "qualification",
        "text_signs",
        "separate_source_glyphs",
        "fused_source_glyphs",
        "text_signs_by_mark",
        "separate_source_glyphs_by_mark",
        "fused_source_glyphs_by_mark",
    }
    if not isinstance(value, dict) or set(value) != fields:
        raise ValueError("edition package waqf fields differ")
    if value["qualification"] != "source-glyph-qualified":
        raise ValueError("edition package waqf qualification differs")
    for field in (
        "text_signs_by_mark",
        "separate_source_glyphs_by_mark",
        "fused_source_glyphs_by_mark",
    ):
        counts = value[field]
        if (
            not isinstance(counts, dict)
            or not counts
            or not all(
                isinstance(mark, str)
                and mark in WAQF_MARKS
                and type(count) is int
                and count > 0
                for mark, count in counts.items()
            )
        ):
            raise ValueError(f"edition package {field} differs")
    text_counts = value["text_signs_by_mark"]
    separate_counts = value["separate_source_glyphs_by_mark"]
    fused_counts = value["fused_source_glyphs_by_mark"]
    if (
        not all(
            type(value[field]) is int and value[field] > 0
            for field in (
                "text_signs",
                "separate_source_glyphs",
                "fused_source_glyphs",
            )
        )
        or value["text_signs"]
        != value["separate_source_glyphs"] + value["fused_source_glyphs"]
        or sum(text_counts.values()) != value["text_signs"]
        or sum(separate_counts.values()) != value["separate_source_glyphs"]
        or sum(fused_counts.values()) != value["fused_source_glyphs"]
        or set(separate_counts) | set(fused_counts) != set(text_counts)
        or any(
            count != separate_counts.get(mark, 0) + fused_counts.get(mark, 0)
            for mark, count in text_counts.items()
        )
    ):
        raise ValueError("edition package waqf counts differ")
    return value


def validate_path_geometry(value: object) -> dict:
    if not isinstance(value, dict) or set(value) != PATH_GEOMETRY_FIELDS:
        raise ValueError("edition package path-geometry fields differ")
    if (
        value["qualification"] != "base-order-preserved"
        or not all(
            type(value[field]) is int and value[field] >= 0
            for field in ("base_paths", "emitted_paths", "restored_paths")
        )
        or value["base_paths"] <= 0
        or value["emitted_paths"] != value["base_paths"] + value["restored_paths"]
        or not all(
            isinstance(value[field], str) and SHA256.fullmatch(value[field])
            for field in ("base_pages_sha256", "emitted_pages_sha256")
        )
    ):
        raise ValueError("edition package path-geometry identity differs")
    return value


def validate_division_sajdah(value: object) -> dict:
    if not isinstance(value, dict) or set(value) != DIVISION_SAJDAH_FIELDS:
        raise ValueError("edition package division/sajdah fields differ")
    if (
        value["qualification"] != "source-owned"
        or not all(
            type(value[field]) is int and value[field] >= 0
            for field in DIVISION_SAJDAH_FIELDS - {"qualification"}
        )
        or value["division_starts"] <= 0
        or value["sajdah_marks"] <= 0
        or value["division_starts"]
        != value["printed_rubu_al_hizb"] + value["unprinted_rubu_al_hizb"]
        or value["sajdah_marks"]
        != value["mapped_sajdah_source_glyphs"] + value["restored_sajdah_source_glyphs"]
    ):
        raise ValueError("edition package division/sajdah counts differ")
    return value


def validate_source_exclusions(value: object, fallback_words: int) -> dict:
    if (
        not isinstance(value, dict)
        or not value
        or any(
            not isinstance(name, str) or not name or type(count) is not int or count < 0
            for name, count in value.items()
        )
        or sum(value.values()) != fallback_words
    ):
        raise ValueError("edition package source exclusions differ")
    return value


def validate_contract(value: object) -> tuple[dict, dict, dict]:
    fields = {
        "schema",
        "schema_version",
        "package_name",
        "edition",
        "print_year_hijri",
        "format_version",
        "source",
        "qvp",
        "converter",
    }
    if not isinstance(value, dict) or set(value) != fields:
        raise ValueError("edition package contract fields differ")
    if (
        value["schema"] != "quran-engine/edition-data-package-contract"
        or value["schema_version"] != 7
        or not isinstance(value["package_name"], str)
        or not value["package_name"]
        or not isinstance(value["edition"], str)
        or not value["edition"]
        or not isinstance(value["print_year_hijri"], int)
        or value["print_year_hijri"] <= 0
        or value["format_version"] != 1
    ):
        raise ValueError("edition package contract identity differs")
    source = value["source"]
    qvp = value["qvp"]
    converter = value["converter"]
    if not all(isinstance(item, dict) for item in (source, qvp, converter)):
        raise TypeError("edition package contract sections must be objects")
    source_fields = {
        "summary_sha256",
        "summary_schema",
        "summary_schema_version",
        "pages_sha256",
        "qualified_geometry_pages_sha256",
        "base_header_qualified_geometry_pages_sha256",
        "base_path_geometry",
        "index_sha256",
        "pages",
        "words",
        "qualification",
        "hq_words",
        "fallback_words",
        "pages_with_hq",
        "base_summary_sha256",
        "base_waqf_source_glyphs_sha256",
        "base_division_sajdah_source_sha256",
        "division_sajdah",
        "source_candidate_tree_sha256",
        "source_manifest_sha256",
        "source_records_sha256",
        "optical_calibration_sha256",
        "source_exclusions",
        "waqf",
    }
    qvp_fields = {
        "tree_sha256",
        "files",
        "pages",
        "word_sidecars",
        "logical_words",
        "atlas_qva_sha256",
        "atlas_json_sha256",
        "surah_names",
    }
    if (
        set(source) != source_fields
        or set(qvp) != qvp_fields
        or set(converter) != {"sha256"}
    ):
        raise ValueError("edition package inventory fields differ")
    digests = (
        source["summary_sha256"],
        source["pages_sha256"],
        source["qualified_geometry_pages_sha256"],
        source["base_header_qualified_geometry_pages_sha256"],
        source["index_sha256"],
        source["base_summary_sha256"],
        source["base_waqf_source_glyphs_sha256"],
        source["base_division_sajdah_source_sha256"],
        source["source_candidate_tree_sha256"],
        source["source_manifest_sha256"],
        source["source_records_sha256"],
        source["optical_calibration_sha256"],
        qvp["tree_sha256"],
        qvp["atlas_qva_sha256"],
        qvp["atlas_json_sha256"],
        converter["sha256"],
    )
    if not all(isinstance(item, str) and SHA256.fullmatch(item) for item in digests):
        raise ValueError("edition package digest differs")
    validate_waqf(source["waqf"])
    validate_path_geometry(source["base_path_geometry"])
    validate_division_sajdah(source["division_sajdah"])
    validate_source_exclusions(source["source_exclusions"], source["fallback_words"])
    if (
        not isinstance(source["summary_schema"], str)
        or not source["summary_schema"]
        or not isinstance(source["summary_schema_version"], int)
        or source["qualification"] != "source-qualified"
        or not all(
            isinstance(count, int) and count > 0
            for count in (
                source["pages"],
                source["words"],
                source["hq_words"],
                source["fallback_words"],
                source["pages_with_hq"],
                qvp["files"],
                qvp["pages"],
                qvp["word_sidecars"],
                qvp["logical_words"],
                qvp["surah_names"],
            )
        )
        or source["hq_words"] + source["fallback_words"] != source["words"]
        or source["pages_with_hq"] > source["pages"]
        or source["pages"] != qvp["pages"]
        or source["words"] != qvp["logical_words"]
    ):
        raise ValueError("edition package counts differ")
    return source, qvp, converter


def verify_source(
    root: Path,
    contract: dict,
    edition: str,
    print_year_hijri: int,
) -> dict:
    summary_path = root / "summary.json"
    if sha256(summary_path) != contract["summary_sha256"]:
        raise ValueError("source summary digest differs")
    summary = read_json(summary_path)
    pages = list(range(1, contract["pages"] + 1))
    page_paths = [root / "pages" / f"{page:03}.svg" for page in pages]
    index_paths = [root / "index/by-page" / f"{page:03}.json" for page in pages]
    if not all(path.is_file() for path in [*page_paths, *index_paths]):
        raise ValueError("source corpus file set is incomplete")
    if (
        not isinstance(summary, dict)
        or summary.get("schema") != contract["summary_schema"]
        or summary.get("schema_version") != contract["summary_schema_version"]
        or summary.get("edition") != edition
        or summary.get("print_year_hijri") != print_year_hijri
        or summary.get("pages") != pages
        or summary.get("qualification") != contract["qualification"]
        or summary.get("counts", {}).get("pages") != contract["pages"]
        or summary.get("counts", {}).get("words") != contract["words"]
        or summary.get("counts", {}).get("hq_words") != contract["hq_words"]
        or summary.get("counts", {}).get("fallback_words") != contract["fallback_words"]
        or summary.get("counts", {}).get("pages_with_hq") != contract["pages_with_hq"]
        or summary.get("base_summary_sha256") != contract["base_summary_sha256"]
        or summary.get("base_waqf_source_glyphs_sha256")
        != contract["base_waqf_source_glyphs_sha256"]
        or summary.get("base_division_sajdah_source_sha256")
        != contract["base_division_sajdah_source_sha256"]
        or summary.get("division_sajdah") != contract["division_sajdah"]
        or summary.get("qualified_geometry_pages_sha256")
        != contract["qualified_geometry_pages_sha256"]
        or summary.get("base_header_qualified_geometry_pages_sha256")
        != contract["base_header_qualified_geometry_pages_sha256"]
        or summary.get("base_path_geometry") != contract["base_path_geometry"]
        or summary.get("waqf") != contract["waqf"]
        or summary.get("source_candidate_tree_sha256")
        != contract["source_candidate_tree_sha256"]
        or summary.get("source_manifest_sha256") != contract["source_manifest_sha256"]
        or summary.get("source_records_sha256") != contract["source_records_sha256"]
        or summary.get("optical_calibration_sha256")
        != contract["optical_calibration_sha256"]
        or summary.get("source_exclusions") != contract["source_exclusions"]
        or summary.get("pages_sha256") != contract["pages_sha256"]
        or summary.get("index_sha256") != contract["index_sha256"]
    ):
        raise ValueError("source corpus identity differs")
    if named_digest(page_paths) != contract["pages_sha256"]:
        raise ValueError("source SVG page digest differs")
    if named_digest(index_paths) != contract["index_sha256"]:
        raise ValueError("source word index digest differs")
    return summary


def expected_qvp_files(root: Path, pages: int, surahs: int) -> list[Path]:
    files = [root / f"{page:03}.qvp" for page in range(1, pages + 1)]
    files += [root / f"{page:03}.words.json" for page in range(1, pages + 1)]
    files += [root / "atlas.qva", root / "atlas.json"]
    files += [
        root / "surah-names/qvp" / f"{surah:03}.qvp" for surah in range(1, surahs + 1)
    ]
    files += [
        root / "surah-names/svg" / f"{surah:03}.svg" for surah in range(1, surahs + 1)
    ]
    files += [
        root / "surah-names/qvp/all.qvp",
        root / "surah-names/svg/all.svg",
        root / "surah-names/surah-names.woff2",
        root / "surah-names/surah-names.css",
        root / "surah-names/map.json",
    ]
    return sorted(files, key=lambda path: path.relative_to(root).as_posix())


def verify_qvp(root: Path, contract: dict) -> list[Path]:
    files = expected_qvp_files(root, contract["pages"], contract["surah_names"])
    actual = sorted(path for path in root.rglob("*") if path.is_file())
    if actual != files or len(files) != contract["files"]:
        raise ValueError("QVP package file set differs")
    if tree_digest(root, files) != contract["tree_sha256"]:
        raise ValueError("QVP package tree digest differs")
    if sha256(root / "atlas.qva") != contract["atlas_qva_sha256"]:
        raise ValueError("QVP atlas digest differs")
    if sha256(root / "atlas.json") != contract["atlas_json_sha256"]:
        raise ValueError("QVP atlas metadata digest differs")
    atlas = read_json(root / "atlas.json")
    if (
        not isinstance(atlas, dict)
        or len(atlas.get("pages", [])) != contract["pages"]
        or len(atlas.get("surahs", [])) != contract["surah_names"]
    ):
        raise ValueError("QVP atlas inventory differs")
    logical_words = 0
    word_key = re.compile(r"^[1-9][0-9]*:[1-9][0-9]*:[1-9][0-9]*$")
    form_fields = {"rasm_uthmani", "rasm_imlai", "qpc", "rasm", "search"}
    for page in range(1, contract["pages"] + 1):
        words = read_json(root / f"{page:03}.words.json")
        if (
            not isinstance(words, dict)
            or not all(
                isinstance(key, str) and word_key.fullmatch(key) for key in words
            )
            or not all(
                isinstance(forms, dict)
                and set(forms) == form_fields
                and all(isinstance(value, str) for value in forms.values())
                for forms in words.values()
            )
        ):
            raise TypeError(f"QVP word sidecar is invalid: {page:03}.words.json")
        logical_words += len(words)
    if logical_words != contract["logical_words"]:
        raise ValueError("QVP logical word count differs")
    return files


def readme(contract: dict, version: str) -> bytes:
    source = contract["source"]
    unprinted = source["division_sajdah"]["unprinted_rubu_al_hizb"]
    boundary = "boundary" if unprinted == 1 else "boundaries"
    return (
        f"# {contract['edition']} {contract['print_year_hijri']}H page data\n\n"
        f"Package: `{contract['package_name']}`\n\n"
        f"Version: `{version}`\n\n"
        "This archive contains the qualified edition corpus encoded as QVP, its canonical "
        "word sidecars, atlas, and Surah-name assets.\n\n"
        f"The corpus contains {source['hq_words']:,} qualified HQ word outlines, plus "
        f"{source['fallback_words']:,} verified QCF/KFGQPC fallback words. "
        f"{source['waqf']['separate_source_glyphs']:,} independently owned waqf signs are "
        f"typed for styling; {source['waqf']['fused_source_glyphs']:,} fused signs remain "
        "deferred. The corpus also records "
        f"{source['division_sajdah']['division_starts']:,} division boundaries, with "
        f"{source['division_sajdah']['printed_rubu_al_hizb']:,} printed rosettes and "
        f"{unprinted:,} metadata-only {boundary}, plus "
        f"{source['division_sajdah']['sajdah_marks']:,} sajdah marks. "
        "The package records the qualified edition geometry and semantic inventories.\n"
    ).encode()


def tar_info(name: str, size: int) -> tarfile.TarInfo:
    info = tarfile.TarInfo(name)
    info.size = size
    info.mode = 0o644
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    return info


def write_archive(
    path: Path, package_name: str, files: dict[str, Path | bytes]
) -> None:
    temporary = path.with_name(path.name + ".tmp")
    if temporary.exists():
        raise ValueError(
            f"temporary archive exists; inspect interrupted write: {temporary}"
        )
    try:
        with tempfile.TemporaryDirectory(dir=path.parent) as directory:
            tar_path = Path(directory) / "payload.tar"
            with tarfile.open(tar_path, "w", format=tarfile.USTAR_FORMAT) as archive:
                for relative, source in sorted(files.items()):
                    name = f"{package_name}/{relative}"
                    if isinstance(source, bytes):
                        archive.addfile(tar_info(name, len(source)), io.BytesIO(source))
                    else:
                        with source.open("rb") as stream:
                            archive.addfile(
                                tar_info(name, source.stat().st_size), stream
                            )
            with (
                tar_path.open("rb") as source,
                temporary.open("wb") as destination,
                gzip.GzipFile(
                    filename="",
                    mode="wb",
                    compresslevel=9,
                    mtime=0,
                    fileobj=destination,
                ) as compressed,
            ):
                shutil.copyfileobj(source, compressed, length=1024 * 1024)
        temporary.replace(path)
    except Exception:
        temporary.unlink(missing_ok=True)
        raise


def package(args: argparse.Namespace) -> dict:
    if not VERSION.fullmatch(args.version):
        raise ValueError("version must be X.Y.Z with an optional prerelease suffix")
    contract = read_json(args.contract)
    source_contract, qvp_contract, converter_contract = validate_contract(contract)
    source_summary = verify_source(
        args.source_dir,
        source_contract,
        contract["edition"],
        contract["print_year_hijri"],
    )
    qvp_files = verify_qvp(args.qvp_dir, qvp_contract)
    if sha256(args.converter) != converter_contract["sha256"]:
        raise ValueError("QVP converter digest differs")
    if not args.license.is_file():
        raise ValueError("package licence is missing")

    archive_name = f"{contract['package_name']}-v{args.version}.tar.gz"
    args.out_dir.mkdir(parents=True, exist_ok=True)
    archive_path = args.out_dir / archive_name
    checksum_path = args.out_dir / f"{archive_name}.sha256"
    if archive_path.exists() or checksum_path.exists():
        raise ValueError("package output already exists")

    payload: dict[str, Path | bytes] = {
        path.relative_to(args.qvp_dir).as_posix(): path for path in qvp_files
    }
    payload["README.md"] = readme(contract, args.version)
    payload["LICENSE.txt"] = args.license
    payload_hashes = {
        name: (
            hashlib.sha256(value).hexdigest()
            if isinstance(value, bytes)
            else sha256(value)
        )
        for name, value in sorted(payload.items())
    }
    version = {
        "schema": "quran-engine/edition-data-release",
        "schema_version": 7,
        "package": contract["package_name"],
        "version": args.version,
        "edition": contract["edition"],
        "print_year_hijri": contract["print_year_hijri"],
        "format_version": contract["format_version"],
        "source": source_contract,
        "qvp": qvp_contract,
        "converter": converter_contract,
        "contract_sha256": sha256(args.contract),
        "source_corpus_manifest_sha256": source_summary.get("source_manifest_sha256"),
        "base_source_manifest_sha256": source_summary.get(
            "base_source_manifest_sha256"
        ),
        "payload_files": payload_hashes,
    }
    payload["VERSION.json"] = json_bytes(version)
    write_archive(archive_path, contract["package_name"], payload)
    archive_sha256 = sha256(archive_path)
    checksum_path.write_text(f"{archive_sha256}  {archive_name}\n")
    report = {
        "package": contract["package_name"],
        "version": args.version,
        "edition": contract["edition"],
        "print_year_hijri": contract["print_year_hijri"],
        "archive": str(archive_path),
        "archive_sha256": archive_sha256,
        "checksum": str(checksum_path),
        "payload_files": len(payload),
        "contract_sha256": version["contract_sha256"],
        "qvp_tree_sha256": qvp_contract["tree_sha256"],
    }
    print(json.dumps(report, indent=2))
    return report


def parser() -> argparse.ArgumentParser:
    command = argparse.ArgumentParser(description=__doc__)
    command.add_argument("version")
    command.add_argument("--source-dir", type=Path, required=True)
    command.add_argument("--qvp-dir", type=Path, required=True)
    command.add_argument("--converter", type=Path, required=True)
    command.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    command.add_argument("--license", type=Path, default=ROOT / "LICENSE")
    command.add_argument("--out-dir", type=Path, default=ROOT / "dist")
    return command


def main() -> None:
    args = parser().parse_args()
    for name in (
        "source_dir",
        "qvp_dir",
        "converter",
        "contract",
        "license",
        "out_dir",
    ):
        setattr(args, name, getattr(args, name).resolve())
    package(args)


if __name__ == "__main__":
    main()
