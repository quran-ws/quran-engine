"""Tests for scripts/ste_lint.py.

Vendored with the script, under the MIT notice in
LICENSES/MIT-ste-plain-writing.txt.
"""

from __future__ import annotations

import importlib.util
import pathlib
import unittest

SCRIPT = pathlib.Path(__file__).parent / "ste_lint.py"
SPEC = importlib.util.spec_from_file_location("ste_lint", SCRIPT)
assert SPEC and SPEC.loader
STE_LINT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STE_LINT)


class LintTests(unittest.TestCase):
    def test_clean_plain_text_has_no_violations(self) -> None:
        result = STE_LINT.lint(
            "The parser reads the file. It reports each invalid field."
        )

        self.assertEqual(result["total"], 0)
        self.assertEqual(result["words"], 10)

    def test_ai_slop_patterns_are_reported(self) -> None:
        result = STE_LINT.lint(
            "It is important to note that our seamless platform is being used "
            "to leverage a variety of powerful tools; furthermore, this "
            "revolutionary approach is designed to facilitate robust outcomes."
        )
        violations = result["violations"]

        self.assertGreater(violations["marketing_adjective"], 0)
        self.assertGreater(violations["banned_word"], 0)
        self.assertGreater(violations["semicolon"], 0)
        self.assertGreater(violations["passive_voice"], 0)
        self.assertGreater(result["total_per100w"], 0)

    def test_code_blocks_do_not_affect_the_score(self) -> None:
        result = STE_LINT.lint(
            "Run this command.\n\n```python\n"
            "perform_an_analysis = 'seamless; robust'\n"
            "```\n"
        )

        self.assertEqual(result["total"], 0)


class DeveloperProseTests(unittest.TestCase):
    def test_slogan_fragments_and_anthropomorphism_are_reported(self) -> None:
        result = STE_LINT.lint(
            "No setup. No surprises. It works. The engine knows every word and "
            "decides the layout, not only fast but also exact."
        )
        violations = result["violations"]

        self.assertGreaterEqual(violations["fragment_run(3+ short)"], 1)
        self.assertGreaterEqual(violations["anthropomorphic_verb"], 2)
        self.assertGreaterEqual(violations["rhetorical_contrast"], 2)
        self.assertGreaterEqual(result["absolute_claims(check, not scored)"], 1)

    def test_leaked_instructions_are_reported(self) -> None:
        result = STE_LINT.lint(
            "As requested, this section covers the cache. Per the plan, the user wants "
            "a table here."
        )

        self.assertGreaterEqual(result["violations"]["leaked_instruction"], 3)

    def test_absolute_claims_are_not_scored(self) -> None:
        result = STE_LINT.lint("The converter is lossless. Every page passes the gate.")

        self.assertEqual(result["violations"]["fragment_run(3+ short)"], 0)
        self.assertEqual(result["total"], 0)


if __name__ == "__main__":
    unittest.main()
