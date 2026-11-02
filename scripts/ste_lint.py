#!/usr/bin/env python3
"""Report mechanical Simplified Technical English and AI-slop signals.

Vendored from the `ste-plain-writing` skill by Ege Celebi and Ryuketsukami, under the
MIT License. The notice is in LICENSES/MIT-ste-plain-writing.txt. `CONTRIBUTING.md`
names this script, so the repository holds it.

The counts are a guide for a writer, not a gate. `scripts/check.sh docs` prints them for
every tracked document. One change from upstream: each row names the file's path, because
this repository holds several files of the same name.
"""

from __future__ import annotations

import glob
import json
import re
import sys

MARKETING = [
    "seamless",
    "seamlessly",
    "robust",
    "powerful",
    "cutting-edge",
    "effortless",
    "effortlessly",
    "world-class",
    "next-generation",
    "revolutionary",
    "blazing",
    "lightning-fast",
    "elegant",
    "delightful",
    "turnkey",
    "best-in-class",
    "state-of-the-art",
    "game-changing",
    "first-class",
    "battle-tested",
    "enterprise-grade",
    "supercharge",
    "unlock",
    "unleash",
    "empower",
    "empowers",
]
BANNED = [
    "begin",
    "begins",
    "commence",
    "commences",
    "initiate",
    "initiates",
    "originate",
    "utilize",
    "utilizes",
    "utilizing",
    "leverage",
    "leverages",
    "leveraging",
    "facilitate",
    "facilitates",
    "ensure",
    "ensures",
    "ensuring",
    "prior to",
    "subsequent to",
    "obtain",
    "obtains",
    "acquire",
    "acquires",
    "demonstrate",
    "demonstrates",
    "additionally",
    "furthermore",
    "moreover",
    "comprehensive",
    "comprehensively",
    "utilization",
    "aforementioned",
    "henceforth",
    "therein",
    "whilst",
    "amongst",
    "numerous",
    "myriad",
    "plethora",
    "in order to",
    "a variety of",
    "in the event that",
    "due to the fact that",
    "it is important to note",
]
PHRASAL = [
    "spin up",
    "spin down",
    "reach out",
    "dive into",
    "dives into",
    "diving into",
    "kick off",
    "kicks off",
    "roll out",
    "rolls out",
    "tear down",
    "ramp up",
    "circle back",
    "drill down",
    "spun up",
    "reaching out",
]
ANTHROPOMORPHIC = [
    "knows", "understands", "wants", "decides", "spends", "cares", "thinks",
    "remembers", "forgets", "believes", "expects", "sees", "trusts", "owns",
]

RHETORICAL_CONTRAST = [
    "not only", "but also", "rather than", "instead of",
]

ABSOLUTE_CLAIM = [
    "lossless", "exact", "identical", "always", "never", "guaranteed", "zero",
]

LEAK_MARKER = [
    "as requested", "as discussed", "as you asked", "you asked", "the user", "this session",
    "the previous session", "earlier session", "per the plan", "in this plan", "the plan file",
    "in the previous step", "in this conversation", "the assistant", "as an ai", "chatgpt",
    "claude code session", "note to self", "for the writer", "in plain words", "in plain english",
    "in simple terms", "easy to follow", "as simply as possible",
]

MODAL_HEDGE = [
    "it is important to note",
    "it should be noted",
    "it is worth noting",
    "please note that",
    "as mentioned",
    "as noted above",
]
BE = r"(?:am|is|are|was|were|be|been|being)"
IRREGULAR_PARTICIPLE = (
    r"(?:done|made|sent|read|built|kept|held|set|put|run|written|shown|given|"
    r"taken|found|got|gotten|seen|known|thrown|drawn)"
)


def strip_code(text: str) -> str:
    text = re.sub(r"```.*?```", " ", text, flags=re.S)
    return re.sub(r"`[^`]*`", " ", text)


def sentences(text: str) -> list[str]:
    output: list[str] = []
    for line in text.splitlines():
        sentence = line.strip()
        if not sentence:
            continue
        sentence = re.sub(r"^\s*#{1,6}\s*", "", sentence)
        sentence = re.sub(r"^\s*(?:[-*+]|\d+[.)])\s+", "", sentence)
        if not sentence:
            continue
        parts = re.split(r"(?<=[.!?:])\s+(?=[A-Z0-9\"'\-])", sentence)
        output.extend(part.strip() for part in parts if part.strip())
    return output


def word_count(text: str) -> int:
    return len(re.findall(r"[A-Za-z0-9][A-Za-z0-9'\-/]*", text))


def fragment_runs(text: str) -> int:
    """Count runs of three or more consecutive short sentences (four words or fewer) in prose.

    Repeated short sentences are the shape of a slogan ("No setup. No surprises. Done.").
    Wrapped prose lines are joined into paragraphs first, so a line break inside a
    sentence does not create a fragment. Table rows, list items and headings are skipped:
    their cells and items are short by design, not for effect.
    """
    paragraphs: list[str] = []
    current: list[str] = []
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or re.match(r"^(?:\||#{1,6}\s|[-*+]\s|\d+[.)]\s)", stripped):
            if current:
                paragraphs.append(" ".join(current))
                current = []
            continue
        current.append(stripped)
    if current:
        paragraphs.append(" ".join(current))
    runs = 0
    for paragraph in paragraphs:
        streak = 0
        for sentence in re.split(r"(?<=[.!?])\s+", paragraph):
            n = word_count(sentence)
            if 0 < n <= 4:
                streak += 1
                if streak == 3:
                    runs += 1
            elif n:
                streak = 0
    return runs


def count_phrases(text: str, phrases: list[str]) -> tuple[int, list[str]]:
    count = 0
    hits: list[str] = []
    lowered = text.lower()
    for phrase in phrases:
        matches = list(
            re.finditer(r"(?<![a-z])" + re.escape(phrase) + r"(?![a-z])", lowered)
        )
        count += len(matches)
        hits.extend(phrase for _ in matches)
    return count, hits


def lint(text: str) -> dict[str, object]:
    raw = text
    text = strip_code(text)
    parsed_sentences = sentences(text)
    words = sum(word_count(sentence) for sentence in parsed_sentences) or 1
    long_sentences = [
        (word_count(sentence), sentence)
        for sentence in parsed_sentences
        if word_count(sentence) > 20
    ]
    violations: dict[str, int] = {
        "long_sentence(>20w)": len(long_sentences),
        "semicolon": text.count(";"),
        "contraction": len(
            re.findall(r"\b\w+['’](?:t|re|ve|ll|d|s|m)\b", text)
        ),
        "passive_voice": len(
            re.findall(
                rf"\b{BE}\s+(?:\w+ed|{IRREGULAR_PARTICIPLE})\b", text, re.I
            )
        ),
        "ing_main_verb": len(re.findall(rf"\b{BE}\s+\w+ing\b", text, re.I)),
        "nominalization": len(
            re.findall(
                r"\b(?:perform(?:s|ed)?|conduct(?:s|ed)?|provide(?:s|d)?|"
                r"carry out|carries out|make use of|makes use of)\b",
                text,
                re.I,
            )
        )
        + len(re.findall(r"\b\w{4,}(?:tion|ment|ance|ence)\s+of\b", text, re.I)),
    }
    violations["phrasal_verb"], _ = count_phrases(text, PHRASAL)
    violations["banned_word"], banned_hits = count_phrases(text, BANNED)
    violations["marketing_adjective"], marketing_hits = count_phrases(text, MARKETING)
    violations["modal_hedge"], _ = count_phrases(text, MODAL_HEDGE)
    violations["anthropomorphic_verb"], anthro_hits = count_phrases(text, ANTHROPOMORPHIC)
    violations["rhetorical_contrast"], _ = count_phrases(text, RHETORICAL_CONTRAST)
    violations["leaked_instruction"], leak_hits = count_phrases(text, LEAK_MARKER)
    violations["fragment_run(3+ short)"] = fragment_runs(text)
    claims, claim_hits = count_phrases(text, ABSOLUTE_CLAIM)
    paragraphs = [p for p in re.split(r"\n\s*\n", raw) if p.strip()]
    violations["long_paragraph(>6s)"] = sum(
        1 for paragraph in paragraphs if len(sentences(strip_code(paragraph))) > 6
    )
    total = sum(violations.values())
    longest_sentence = max(
        (word_count(sentence) for sentence in parsed_sentences), default=0
    )
    return {
        "words": words,
        "sentences": len(parsed_sentences),
        "violations": violations,
        "total": total,
        "total_per100w": round(total * 100.0 / words, 2),
        "em_dash(slop-marker)": raw.count("—") + raw.count("–"),
        "longest_sentence_words": longest_sentence,
        "sample_marketing": list(dict.fromkeys(marketing_hits))[:6],
        "sample_banned": list(dict.fromkeys(banned_hits))[:6],
        "sample_anthropomorphic": list(dict.fromkeys(anthro_hits))[:8],
        "sample_leaked": list(dict.fromkeys(leak_hits))[:8],
        "absolute_claims(check, not scored)": claims,
        "sample_absolute_claims": list(dict.fromkeys(claim_hits))[:8],
    }


def main(arguments: list[str]) -> int:
    if not arguments:
        print(json.dumps(lint(sys.stdin.read()), indent=2))
        return 0

    paths: list[str] = []
    for argument in arguments:
        if any(character in argument for character in "*?["):
            paths.extend(sorted(glob.glob(argument)))
        else:
            paths.append(argument)

    for path in paths:
        with open(path, encoding="utf-8") as handle:
            result = lint(handle.read())
        print(
            f"{path:52} "
            f"words={result['words']:4d} "
            f"total={result['total']:3d} "
            f"per100w={result['total_per100w']:6.2f} "
            f"em_dash={result['em_dash(slop-marker)']:2d}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
