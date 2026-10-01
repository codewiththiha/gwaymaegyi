import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from check_paths import find_collisions


class PathTests(unittest.TestCase):
    def test_case_only_files_collide(self):
        self.assertEqual(find_collisions(["agents.md", "AGENTS.md"]), [("agents.md", "AGENTS.md")])

    def test_directory_aliases_collide_even_with_different_children(self):
        self.assertEqual(find_collisions(["src/Board/a.rs", "src/board/b.rs"]), [("src/Board", "src/board")])

    def test_shared_directory_prefixes_do_not_collide(self):
        self.assertEqual(find_collisions(["src/board/a.rs", "src/board/b.rs"]), [])

    def test_unicode_case_aliases_collide(self):
        self.assertEqual(find_collisions(["assets/Ä.png", "assets/ä.png"]), [("assets/Ä.png", "assets/ä.png")])
