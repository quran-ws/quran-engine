#!/usr/bin/env python3
"""Check that the C header, the Rust exports and every wrapper agree, and write the parity table.

Inputs
  crates/qvp-ffi/include/qvp.h                 the contract
  crates/qvp-ffi/src/lib.rs                    the #[no_mangle] exports
  packages/android/qvp/src/main/cpp/qvp.h      the Android copy of the header
  the wrapper sources listed in WRAPPERS
  the published JavaScript entry points listed in JS_SURFACES
  docs/API-PARITY.md                           the "Declared gaps" table (symbol, wrapper, issue)

Output
  docs/API-PARITY.md, regenerated between the parity markers (default), or checked
  against the current sources (--check). Exit 1 on: a header/Rust mismatch, a stale
  Android header copy, a symbol that a wrapper neither binds nor declares as a gap, or an
  abbreviated field name in a published JavaScript entry point.

A symbol counts as bound in a wrapper when the C name or its camelCase form (with or
without the object-model noun) appears in that wrapper's sources.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HEADER = ROOT / "crates/qvp-ffi/include/qvp.h"
RUST = ROOT / "crates/qvp-ffi/src/lib.rs"
ANDROID_HEADER = ROOT / "packages/android/qvp/src/main/cpp/qvp.h"
PARITY = ROOT / "docs/API-PARITY.md"
BEGIN, END = "<!-- parity:begin -->", "<!-- parity:end -->"

WRAPPERS = {
    "web": ["web/qvp.js", "web/index.mjs"],
    "android": ["packages/android/qvp/src/main/cpp/qvp_jni.c", "packages/android/qvp/src/main/kotlin"],
    "flutter": ["packages/flutter/qvp_flutter/lib"],
    "ios": ["packages/ios/QvpKit/Sources"],
    "react-native": ["packages/react-native/qvp-react-native/src", "packages/react-native/qvp-react-native/android/src"],
}


def header_symbols() -> list[str]:
    text = HEADER.read_text()
    return sorted(set(re.findall(r"\b(qvp_[a-z0-9_]+)\s*\(", text)))


def rust_exports() -> list[str]:
    text = RUST.read_text()
    return sorted(set(re.findall(r'#\[no_mangle\]\s*pub\s+(?:unsafe\s+)?extern\s+"C"\s+fn\s+(qvp_[a-z0-9_]+)', text)))


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
    "flutter": "packages/flutter/qvp_flutter/lib/src/engine.dart",
    "ios": "packages/ios/QvpKit/Sources/QvpKit/Types.swift",
}


def number(text: str) -> float:
    text = text.rstrip("uUfF").rstrip("L")
    return float(int(text, 16)) if text.lower().startswith("0x") else float(text)


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
                problems.append(f"{site} ({path}) has {name} = {got[name]}, the header says {value}")


def main(argv: list[str]) -> int:
    check = "--check" in argv
    problems: list[str] = []
    check_defaults(problems)
    check_js_names(problems)

    symbols = header_symbols()
    exports = rust_exports()
    if symbols != exports:
        only_h = sorted(set(symbols) - set(exports))
        only_rs = sorted(set(exports) - set(symbols))
        problems.append(f"header and Rust exports differ: only in header {only_h}, only in Rust {only_rs}")

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
