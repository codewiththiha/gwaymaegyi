"""Test language-specific module summaries without treating ordinary code as a header."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from check_headers import has_header


class HeaderTests(unittest.TestCase):
    def test_rust_module_docs(self):
        self.assertTrue(has_header("core.rs", "//! Owns chess rules.\nmod board;"))
        self.assertFalse(has_header("core.rs", "// ordinary note\nmod board;"))

    def test_python_docstrings_and_shebang(self):
        self.assertTrue(has_header("tool.py", '#!/usr/bin/env python3\n"""Validate inputs."""\n'))
        self.assertFalse(has_header("tool.py", "import sys\n"))

    def test_javascript_comments(self):
        self.assertTrue(has_header("worker.mjs", "// Owns worker scheduling.\n"))
        self.assertTrue(has_header("test.cjs", "/** Tests compiled bindings. */\n"))
        self.assertFalse(has_header("worker.mjs", "import init from './pkg.js';\n"))
