import unittest
import re
import tempfile
from pathlib import Path

import src.config as config
from src.agent_integrations import install_antigravity

class TestSkillPackagingAndCompliance(unittest.TestCase):
    def setUp(self):
        self.skill_path = config.BASE_DIR / "skills" / "experienced-llm" / "SKILL.md"

    def test_skill_file_exists(self):
        self.assertTrue(self.skill_path.exists(), f"SKILL.md not found at {self.skill_path}")

    def test_yaml_frontmatter_specification(self):
        content = self.skill_path.read_text(encoding="utf-8")
        match = re.match(r"^---\n(.*?)\n---", content, re.DOTALL)
        self.assertIsNotNone(match, "SKILL.md must start with YAML frontmatter bounded by ---")

        frontmatter = match.group(1)

        # 1. Name
        name_match = re.search(r"^name:\s*([\w\-]+)", frontmatter, re.MULTILINE)
        self.assertIsNotNone(name_match, "Missing 'name' in YAML frontmatter")
        self.assertEqual(name_match.group(1), "experienced-llm")

        # 2. Description (< 200 chars)
        desc_match = re.search(r"^description:\s*(.+)$", frontmatter, re.MULTILINE)
        self.assertIsNotNone(desc_match, "Missing 'description' in YAML frontmatter")
        desc = desc_match.group(1).strip()
        self.assertLess(len(desc), 200, f"Description must be < 200 chars, got {len(desc)}: '{desc}'")
        self.assertGreater(len(desc), 20, "Description must be descriptive (> 20 chars)")

        # 3. Version
        ver_match = re.search(r"^version:\s*(.+)$", frontmatter, re.MULTILINE)
        self.assertIsNotNone(ver_match, "Missing 'version' in YAML frontmatter")
        self.assertEqual(ver_match.group(1).strip(), "1.0.0rc4")

        # 4. Author
        author_match = re.search(r"^author:\s*(.+)$", frontmatter, re.MULTILINE)
        self.assertIsNotNone(author_match, "Missing 'author' in YAML frontmatter")
        self.assertEqual(author_match.group(1).strip(), "developer88")

        # 5. Repository
        repo_match = re.search(r"^repository:\s*(.+)$", frontmatter, re.MULTILINE)
        self.assertIsNotNone(repo_match, "Missing 'repository' in YAML frontmatter")
        self.assertEqual(repo_match.group(1).strip(), "https://github.com/developer88/experienced-llms")

        # 6. License
        lic_match = re.search(r"^license:\s*(.+)$", frontmatter, re.MULTILINE)
        self.assertIsNotNone(lic_match, "Missing 'license' in YAML frontmatter")
        self.assertEqual(lic_match.group(1).strip(), "MIT")

        # 7. Keywords
        self.assertIn("keywords:", frontmatter)
        self.assertIn("memory", frontmatter)
        self.assertIn("persistent-memory", frontmatter)

    def test_operational_documentation_sections(self):
        content = self.skill_path.read_text(encoding="utf-8")
        self.assertIn("Startup Bootstrap", content)
        self.assertIn("Daytime Operational Logging", content)
        self.assertIn("Consolidation Engine", content)
        self.assertIn("experienced-llms log", content)
        self.assertIn("experienced-llms consolidate", content)
        self.assertIn("--prepare", content)
        self.assertIn("--apply", content)

    def test_install_antigravity_integration(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            target_dir = Path(tmp_dir) / "test_skill"
            ok, msg = install_antigravity(target_dir=target_dir)
            self.assertTrue(ok)
            installed_file = target_dir / "SKILL.md"
            self.assertTrue(installed_file.exists())
            installed_content = installed_file.read_text(encoding="utf-8")
            self.assertIn("name: experienced-llm", installed_content)
            self.assertIn("version: 1.0.0rc4", installed_content)

if __name__ == "__main__":
    unittest.main()
