"""Test Conventional Commit validation and subject/body boundaries."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from check_commit import validate


class CommitTests(unittest.TestCase):
    def test_supported_forms(self):
        for message in ["feat(core): add typed squares", "ci: check wasm", "feat(core)!: change move encoding\n\nExplain the interface change.\n"]:
            self.assertEqual(validate(message), [])

    def test_length_counts_the_complete_unicode_subject(self):
        self.assertEqual(validate("docs: " + "é" * 66), [])
        self.assertTrue(validate("docs: " + "é" * 67))

    def test_malformed_subjects_and_separators(self):
        for message in ["update files", "fix(core): correct parsing.", "fix(core): correct parsing\nbody", "fix(core): correct parsing\n\n\nbody"]:
            self.assertTrue(validate(message), message)


if __name__ == "__main__":
    unittest.main()
