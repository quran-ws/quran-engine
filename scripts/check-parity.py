#!/usr/bin/env python3
"""Check that the C header, the Rust exports and every wrapper agree, and write the parity table.

Inputs
  crates/qvp-ffi/include/qvp.h                 the contract
  crates/qvp-ffi/src/*.rs                      the #[no_mangle] exports, one module per header section
  packages/android/qvp/src/main/cpp/qvp.h      the Android copy of the header
  the wrapper sources listed in WRAPPERS
  the published JavaScript entry points listed in JS_SURFACES
  docs/API-PARITY.md                           the "Declared gaps" table (symbol, wrapper, issue)
  the React Native target and selector codes   written by scripts/gen-rn-enums.py

Output
  docs/API-PARITY.md, regenerated between the parity markers (default), or checked
  against the current sources (--check). Exit 1 on: a header/Rust mismatch, a stale
  Android header copy, a symbol that a wrapper neither binds nor declares as a gap, a
  header function without a row in the "Every symbol" table of docs/API.md (or a row
  without a function), a wrapper that leaves out a QVP_KIND_*, QVP_FAMILY_*,
  QVP_CATEGORY_* or QVP_DECORATION_* code or gives it another value, an abbreviated field name in a published JavaScript entry point, or
  React Native code files that differ from what scripts/gen-rn-enums.py writes from the
  header.

A symbol counts as bound in a wrapper when the C name or its camelCase form (with or
without the object-model noun) appears in that wrapper's sources. A file generated from
the header (GENERATED) names every symbol, so it does not count: the wrapper must call it.
"""
from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HEADER = ROOT / "crates/qvp-ffi/include/qvp.h"
WEB_QVP_JS = ROOT / "web/qvp.js"
RUST_SRC = ROOT / "crates/qvp-ffi/src"
ANDROID_HEADER = ROOT / "packages/android/qvp/src/main/cpp/qvp.h"
API_DOC = ROOT / "docs/API.md"
PARITY = ROOT / "docs/API-PARITY.md"
BEGIN, END = "<!-- parity:begin -->", "<!-- parity:end -->"

WRAPPERS = {
    "web": ["web/qvp.js", "web/index.mjs"],
    "android": ["packages/android/qvp/src/main/cpp/qvp_jni.c", "packages/android/qvp/src/main/kotlin"],
    "flutter": ["packages/flutter/qvp_flutter/lib"],
    "ios": ["packages/ios/QvpKit/Sources"],
    "react-native": ["packages/react-native/qvp-react-native/src", "packages/react-native/qvp-react-native/android/src"],
}
# Bindings generated from the header. scripts/check-flutter-bindings.sh keeps the Flutter
# file in step with the header; this script skips it.
GENERATED = {"packages/flutter/qvp_flutter/lib/src/bindings.dart"}


def header_symbols() -> list[str]:
    text = HEADER.read_text()
    return sorted(set(re.findall(r"\b(qvp_[a-z0-9_]+)\s*\(", text)))


def rust_exports() -> list[str]:
    text = "\n".join(f.read_text() for f in sorted(RUST_SRC.glob("*.rs")))
    return sorted(set(re.findall(r'#\[no_mangle\]\s*pub\s+(?:unsafe\s+)?extern\s+"C"\s+fn\s+(qvp_[a-z0-9_]+)', text)))


def check_api_table(symbols: list[str], problems: list[str]) -> None:
    """docs/API.md, "Every symbol": one row per header function, none for a function the header lacks."""
    text = API_DOC.read_text()
    start = text.find("## Every symbol")
    if start < 0:
        problems.append("docs/API.md has no 'Every symbol' section")
        return
    end = text.find("\n## ", start + 1)
    table = text[start:end if end >= 0 else len(text)]
    rows = re.findall(r"^\|[^|]+\|\s*`(qvp_[a-z0-9_]+)`\s*\|", table, re.M)
    for s in sorted(set(symbols) - set(rows)):
        problems.append(f"docs/API.md 'Every symbol' has no row for `{s}`")
    for s in sorted(set(rows) - set(symbols)):
        problems.append(f"docs/API.md 'Every symbol' has a row for `{s}`, which the header lacks")
    for s in sorted({r for r in rows if rows.count(r) > 1}):
        problems.append(f"docs/API.md 'Every symbol' has more than one row for `{s}`")


def camel(symbol: str, drop_noun: bool) -> str:
    """qvp_atlas_page_of -> atlasPageOf, or pageOf when the receiver is the atlas object."""
    parts = symbol.split("_")[1:]
    if drop_noun and parts and parts[0] == "atlas" and len(parts) > 1:
        parts = parts[1:]
    return parts[0] + "".join(p.capitalize() for p in parts[1:])


def source_text(paths: list[str]) -> str:
    chunks = []
    for rel in paths:
        p = ROOT / rel
        files = [p] if p.is_file() else sorted(q for q in p.rglob("*") if q.is_file())
        for f in files:
            if f.relative_to(ROOT).as_posix() in GENERATED:
                continue
            try:
                chunks.append(f.read_text())
            except UnicodeDecodeError:
                pass
    return "\n".join(chunks)


def bound(symbol: str, text: str) -> bool:
    for name in (symbol, camel(symbol, False), camel(symbol, True)):
        if re.search(rf"\b{re.escape(name)}\b", text):
            return True
    return False


def declared_gaps() -> dict[tuple[str, str], str]:
    gaps: dict[tuple[str, str], str] = {}
    if not PARITY.exists():
        return gaps
    section = PARITY.read_text().split("## Declared gaps", 1)
    if len(section) < 2:
        return gaps
    for line in section[1].splitlines():
        m = re.match(r"\|\s*`?(qvp_[a-z0-9_]+)`?\s*\|\s*([a-z-]+)\s*\|\s*([^|]*?)\s*\|", line)
        if m:
            gaps[(m.group(1), m.group(2))] = m.group(3)
    return gaps


# Every entry point of the npm package returns the same field names. The lite decoder
# reads the format directly and binds no qvp_* symbol, so nothing else here covers it.
JS_SURFACES = ["web/qvp.js", "web/index.mjs", "web/lite.mjs"]
# The abbreviations docs/standards/NAMING.md replaced, as whole identifiers.
BANNED_JS_NAMES = {
    "idx": "index", "lineIdx": "lineIndex", "ayahIdx": "ayahIndex", "wordIdx": "wordIndex",
    "nDecos": "nDecorations", "deco": "decoration", "decos": "decorations",
    "lineNo": "lineNumber", "ayahNo": "ayahNumber",
}


def check_js_names(problems: list[str]) -> None:
    for rel in JS_SURFACES:
        path = ROOT / rel
        if not path.exists():
            problems.append(f"{rel} is listed as a published entry point but does not exist")
            continue
        text = path.read_text()
        for old, new in BANNED_JS_NAMES.items():
            if re.search(rf"\b{old}\b", text):
                problems.append(f"{rel} uses `{old}`; the name is `{new}` (docs/standards/NAMING.md)")


DEFAULT_SITES = {
    "core": "crates/qvp-core/src/defaults.rs",
    "web": "web/qvp.js",
    "android": "packages/android/qvp/src/main/kotlin/ws/quran/qvp/Types.kt",
    "flutter": "packages/flutter/qvp_flutter/lib/src/types.dart",
    "ios": "packages/ios/QvpKit/Sources/QvpKit/Types.swift",
}


def number(text: str) -> float:
    """A header or wrapper literal as a number. A suffix is stripped by literal form: `u`/`L`
    after a hex colour, `f`/`u`/`L` after a decimal. A hex digit f is part of the number, so
    `0x231f20ffu` is 0x231f20ff, not 0x231f20."""
    if text.lower().startswith("0x"):
        return float(int(text[2:].rstrip("uUlL"), 16))
    return float(text.rstrip("uUfFlL"))


def header_defaults() -> dict[str, float]:
    return {m.group(1): number(m.group(2)) for m in re.finditer(r"#define QVP_DEFAULT_([A-Z_]+)\s+(0x[0-9a-fA-F]+u?|[0-9.]+f?)", HEADER.read_text())}


def defaults_in(path: str, names: list[str]) -> dict[str, float | None]:
    """Each default's value in a source file: NAME (any case, camel or snake) = or : value."""
    text = (ROOT / path).read_text()
    # Search from the defaults block on, not from the top of the file.
    for marker in ("QvpDefaults", "DEFAULTS = "):
        if marker in text:
            text = text[text.index(marker):]
            break
    out: dict[str, float | None] = {}
    for name in names:
        camel = name.split("_")[0].lower() + "".join(p.capitalize() for p in name.split("_")[1:])
        pattern = rf"\b(?:{name}|{camel})\b(?:\s*:\s*[A-Za-z0-9<>]+)?\s*[=:]\s*(0x[0-9a-fA-F]+|[0-9.]+)"
        m = re.search(pattern, text)
        out[name] = number(m.group(1)) if m else None
    return out


def check_defaults(problems: list[str]) -> None:
    want = header_defaults()
    if not want:
        problems.append("no QVP_DEFAULT_* in the header")
        return
    for site, path in DEFAULT_SITES.items():
        got = defaults_in(path, list(want))
        for name, value in want.items():
            if got[name] is None:
                problems.append(f"{site} ({path}) has no default named {name}")
            elif abs(got[name] - value) > 1e-9:
                problems.append(f"{site} ({path}) has default {name} = {got[name]}, the header says {value}")
    check_default_tables(want, problems)


# The whole-table forms of the defaults, by camelCase name, that the React Native modules send
# as constants: Kotlin's QvpDefaults.all and Swift's QvpDefaults.decimal. Each entry names its
# constant, so the values are the ones checked above; this checks that no default is left out.
DEFAULT_TABLES = {
    "android": ("packages/android/qvp/src/main/kotlin/ws/quran/qvp/Types.kt", r'"([a-zA-Z]+)" to ([A-Z_]+)'),
    "ios": ("packages/ios/QvpKit/Sources/QvpKit/Types.swift", r'"([a-zA-Z]+)": ([A-Z_]+)'),
}


def check_default_tables(want: dict[str, float], problems: list[str]) -> None:
    for site, (path, pattern) in DEFAULT_TABLES.items():
        text = (ROOT / path).read_text()
        table = dict(re.findall(pattern, text[text.index("QvpDefaults"):]))
        for name in want:
            camel = name.split("_")[0].lower() + "".join(p.capitalize() for p in name.split("_")[1:])
            if table.get(camel) != name:
                problems.append(f"{site} ({path}) has no `{camel}` for {name} in its defaults table")


def gen_rn_enums():
    """scripts/gen-rn-enums.py as a module: its header enum reader and its writers."""
    spec = importlib.util.spec_from_file_location("gen_rn_enums", ROOT / "scripts/gen-rn-enums.py")
    gen = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(gen)
    return gen


# The header enums every wrapper names, by prefix: (the JavaScript object, the native class).
ENUMS = {
    "KIND": ("KIND", "QvpKind"),
    "FAMILY": ("FAMILY", "QvpFamily"),
    "CATEGORY": ("CATEGORY", "QvpCategory"),
    "DECORATION": ("DECORATION", "QvpDecorationKind"),
}
# Where each wrapper declares them, and how a declaration starts: {js} or {cls} is the name.
ENUM_SITES = {
    "web": ("web/qvp.js", "const {js} = {{"),
    "react-native": ("packages/react-native/qvp-react-native/src/index.tsx", "const {js} = {{"),
    "android": ("packages/android/qvp/src/main/kotlin/ws/quran/qvp/Types.kt", "object {cls} {{"),
    "flutter": ("packages/flutter/qvp_flutter/lib/src/types.dart", "class {cls} {{"),
    "ios": ("packages/ios/QvpKit/Sources/QvpKit/Types.swift", "enum {cls} {{"),
}


# The tokens of an enum block that are not code: a string literal, a `/* */` block comment
# or a `//` line comment (Dart's `///` too), whichever starts first, and the `}` that ends
# the block.
TOKEN = re.compile(r""""(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*'|/\*.*?\*/|//[^\n]*|\}""", re.S)


def enum_body(tail: str) -> str:
    """The code of an enum block, from just after its opening `{` to its closing `}`. Its
    comments are removed and its strings emptied, so neither a commented-out code nor a
    `//` or `}` inside a string counts."""
    out, at = [], 0
    for m in TOKEN.finditer(tail):
        out.append(tail[at:m.start()])
        at = m.end()
        token = m.group()
        if token == "}":
            return "".join(out)
        if token[0] in "\"'":
            out.append(token[0] * 2)
    raise ValueError("the enum block has no closing `}`")


def check_enums(problems: list[str]) -> None:
    """Every wrapper names every code of each enum in ENUMS, at the header's value, and no
    code the header lacks. A declaration runs from its start to the first `}` outside a
    comment or a string (`enum_body`), so a commented-out code is not counted. A code is its SUFFIX or camelCase name followed by `=` or `:` and the value."""
    gen = gen_rn_enums()
    header = HEADER.read_text()
    for prefix, (js, cls) in ENUMS.items():
        codes = gen.enum(prefix, header)
        for site, (path, start) in ENUM_SITES.items():
            text = (ROOT / path).read_text()
            marker = start.format(js=js, cls=cls)
            if marker not in text:
                problems.append(f"{site} ({path}) has no `{marker}` for QVP_{prefix}_*")
                continue
            body = enum_body(text[text.index(marker) + len(marker):])
            known = {n for suffix, _ in codes for n in (suffix, gen.camel(suffix))}
            for name in re.findall(r"\b([A-Za-z_]\w*)\s*[=:]\s*\d+", body):
                if name not in known:
                    problems.append(f"{site} ({path}) names {name} in {marker.rstrip('{= ')}, which the header has no QVP_{prefix}_* for")
            for suffix, value in codes:
                m = re.search(rf"\b(?:{suffix}|{gen.camel(suffix)})\s*[=:]\s*(\d+)", body)
                if not m:
                    problems.append(f"{site} ({path}) does not name QVP_{prefix}_{suffix}")
                elif int(m.group(1)) != value:
                    problems.append(f"{site} ({path}) has QVP_{prefix}_{suffix} = {m.group(1)}, the header says {value}")


def check_rn_codes(problems: list[str]) -> None:
    """The React Native code files match what scripts/gen-rn-enums.py writes from the header."""
    gen = gen_rn_enums()
    header = HEADER.read_text()
    tables = {prefix: gen.enum(prefix, header) for prefix in gen.TABLES}
    for path, text in gen.outputs(tables):
        if not path.exists() or path.read_text() != text:
            problems.append(f"{path.relative_to(ROOT)} is out of date; run scripts/gen-rn-enums.py")


def main(argv: list[str]) -> int:
    check = "--check" in argv
    problems: list[str] = []
    check_defaults(problems)
    check_enums(problems)
    check_js_names(problems)
    check_rn_codes(problems)

    symbols = header_symbols()
    exports = rust_exports()
    if symbols != exports:
        only_h = sorted(set(symbols) - set(exports))
        only_rs = sorted(set(exports) - set(symbols))
        problems.append(f"header and Rust exports differ: only in header {only_h}, only in Rust {only_rs}")

    check_api_table(symbols, problems)

    if ANDROID_HEADER.read_bytes() != HEADER.read_bytes():
        problems.append(f"{ANDROID_HEADER.relative_to(ROOT)} differs from {HEADER.relative_to(ROOT)}; run scripts/build-engine-android.sh")

    texts = {w: source_text(paths) for w, paths in WRAPPERS.items()}
    gaps = declared_gaps()
    rows = []
    undeclared = []
    for s in symbols:
        cells = []
        for w in WRAPPERS:
            if bound(s, texts[w]):
                cells.append("yes")
            elif (s, w) in gaps:
                cells.append(f"gap ({gaps[(s, w)]})")
            else:
                cells.append("**missing**")
                undeclared.append((s, w))
        rows.append(f"| `{s}` | " + " | ".join(cells) + " |")

    for s, w in undeclared:
        problems.append(f"{w} neither binds nor declares `{s}`")

    table = "\n".join(
        [f"| symbol | {' | '.join(WRAPPERS)} |", "|---|" + "---|" * len(WRAPPERS)] + rows
    )
    summary = (
        f"{len(symbols)} symbols in the header, {len(exports)} Rust exports. "
        + ", ".join(f"{w}: {sum(1 for s in symbols if bound(s, texts[w]))} bound" for w in WRAPPERS)
        + "."
    )
    block = f"{BEGIN}\n{summary}\n\n{table}\n{END}"

    current = PARITY.read_text() if PARITY.exists() else ""
    if BEGIN in current and END in current:
        new = current[: current.index(BEGIN)] + block + current[current.index(END) + len(END):]
    else:
        new = current.rstrip() + "\n\n" + block + "\n"

    if check:
        if new != current:
            problems.append("docs/API-PARITY.md is out of date; run scripts/check-parity.py")
    else:
        PARITY.write_text(new)

    print(summary + f" {len(header_defaults())} defaults agree across the header, the core and the wrappers." if not any("default" in p for p in problems) else summary)
    for p in problems:
        print("FAIL", p)
    if not problems:
        print("ok    every symbol is bound or declared in every wrapper")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
