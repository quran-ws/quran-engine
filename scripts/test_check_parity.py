"""Tests for the enum reader of scripts/check-parity.py."""

from __future__ import annotations

import importlib.util
import pathlib
import unittest

SCRIPT = pathlib.Path(__file__).parent / "check-parity.py"
SPEC = importlib.util.spec_from_file_location("check_parity", SCRIPT)
assert SPEC and SPEC.loader
PARITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PARITY)


class EnumBodyTests(unittest.TestCase):
    def test_a_commented_out_code_is_removed(self) -> None:
        body = PARITY.enum_body(" a = 0, // b = 1,\n /* c = 2, */ /// d = 3\n e = 4 } f = 5")
        self.assertNotIn("b", body)
        self.assertNotIn("c", body)
        self.assertNotIn("d", body)
        self.assertIn("e = 4", body)
        self.assertNotIn("f", body)

    def test_a_string_keeps_its_slashes_and_braces_out(self) -> None:
        body = PARITY.enum_body(' url: "https://x/*y}", x: 1, z: \'a//b\', w: 2 } v: 3')
        self.assertIn("x: 1", body)
        self.assertIn("w: 2", body)
        self.assertNotIn("v", body)
        self.assertNotIn("https", body)

    def test_a_comment_holds_a_quote_and_a_brace(self) -> None:
        body = PARITY.enum_body(" /// the page's } kind\n a = 0 }")
        self.assertEqual(body.strip(), "a = 0")

    def test_an_unclosed_block_is_an_error(self) -> None:
        with self.assertRaises(ValueError):
            PARITY.enum_body(" a = 0, b = 1")


if __name__ == "__main__":
    unittest.main()
