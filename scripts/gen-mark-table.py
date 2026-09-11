#!/usr/bin/env python3
"""Regenerate the `is_mark` ranges in crates/qvp-core/src/text.rs.

The shared search-fold spec defines a mark by Unicode general category
(Mn Me Lm Sk So Cf). qvp-core has no dependencies and will not carry a Unicode
category table, so the Arabic-relevant subset is written out as ranges. This
script prints them; paste the result into `is_mark`.
"""
import unicodedata

CATEGORIES = {"Mn", "Me", "Lm", "Sk", "So", "Cf"}
SPANS = [(0x0600, 0x06FF), (0x0750, 0x077F), (0x08A0, 0x08FF), (0xFB50, 0xFDFF),
         (0xFE70, 0xFEFF), (0x200B, 0x200F), (0x2060, 0x2064), (0x10E60, 0x10E7E)]

points = {c for a, b in SPANS for c in range(a, b + 1)
          if unicodedata.category(chr(c)) in CATEGORIES}

ranges, start, prev = [], None, None
for c in sorted(points):
    if start is None:
        start = prev = c
    elif c == prev + 1:
        prev = c
    else:
        ranges.append((start, prev))
        start = prev = c
ranges.append((start, prev))

terms = [f"0x{a:04X}..=0x{b:04X}" if a != b else f"0x{a:04X}" for a, b in ranges]
print(f"// {len(points)} codepoints, {len(ranges)} ranges")
line = ""
for i, t in enumerate(terms):
    piece = t + (" | " if i < len(terms) - 1 else "")
    if len(line) + len(piece) > 92:
        print("        " + line.rstrip())
        line = ""
    line += piece
print("        " + line.rstrip())
