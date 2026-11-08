"""Tests for scripts/bump_version.py. Run: python3 -m unittest discover -s scripts -p "test_*.py"."""

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import bump_version as bv  # noqa: E402

PACKAGE = '{\n  "name": "wishtory-tools",\n  "version": "{v}",\n  "type": "module"\n}\n'
PUBSPEC = "name: wishtory\nversion: {v}+7\nenvironment:\n  sdk: ^3.13.0\n"
CARGO = '[workspace]\nmembers = ["a"]\n\n[workspace.package]\nversion = "{v}"\n\n[profile.release]\nopt-level = "s"\n'
LOCK = (
    'version = 4\n\n[[package]]\nname = "serde"\nversion = "1.0.1"\n\n'
    '[[package]]\nname = "wishtory-bridge"\nversion = "{v}"\n\n'
    '[[package]]\nname = "wishtory-core"\nversion = "{v}"\n'
)


class Repo(unittest.TestCase):
    """A throw-away copy of the version files, with the script pointed at it."""

    newline = "\n"

    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        root = Path(self.dir.name)
        files = {
            "package.json": PACKAGE,
            "pubspec.yaml": PUBSPEC,
            "Cargo.toml": CARGO,
            "Cargo.lock": LOCK,
        }
        self.paths = {}
        for name, template in files.items():
            path = root / name
            path.write_bytes(template.replace("{v}", "0.2.1").replace("\n", self.newline).encode())
            self.paths[name] = path
        self.saved = bv.TARGETS
        names = ["package.json", "pubspec.yaml", "Cargo.toml", "Cargo.lock"]
        bv.TARGETS = [(self.paths[name], label, pattern, expected) for name, (_, label, pattern, expected) in zip(names, bv.TARGETS)]

    def tearDown(self):
        bv.TARGETS = self.saved
        self.dir.cleanup()

    def text(self, name):
        return self.paths[name].read_bytes().decode()


class Bump(Repo):
    def test_sets_every_file_and_keeps_the_flutter_build_number(self):
        self.assertEqual(bv.bump("0.3.0"), 0)
        self.assertIn('"version": "0.3.0"', self.text("package.json"))
        self.assertIn("version: 0.3.0+7", self.text("pubspec.yaml"))
        self.assertRegex(self.text("Cargo.toml"), r'\[workspace\.package\]\r?\nversion = "0\.3\.0"')
        lock = self.text("Cargo.lock")
        self.assertEqual(lock.count('version = "0.3.0"'), 2)
        self.assertRegex(lock, r'name = "serde"\r?\nversion = "1\.0\.1"')

    def test_check_agrees_after_a_bump_and_fails_before(self):
        self.assertEqual(bv.check("v0.2.1"), 0)
        self.assertEqual(bv.check("v0.3.0"), 1)
        bv.bump("0.3.0")
        self.assertEqual(bv.check("v0.3.0"), 0)

    def test_a_pre_release_version_is_accepted(self):
        self.assertEqual(bv.bump("0.3.0-rc1"), 0)
        self.assertIn("version: 0.3.0-rc1+7", self.text("pubspec.yaml"))
        self.assertEqual(bv.check("v0.3.0-rc1"), 0)

    def test_a_bad_version_changes_nothing(self):
        self.assertEqual(bv.bump("three"), 1)
        self.assertEqual(bv.check("v0.2.1"), 0)

    def test_current_version_is_read_from_package_json(self):
        self.assertEqual(bv.current_version(), "0.2.1")


class BumpWithWindowsLineEndings(Bump):
    newline = "\r\n"

    def test_line_endings_survive(self):
        bv.bump("0.3.0")
        for name in self.paths:
            text = self.text(name)
            self.assertNotIn("\r\r", text)
            self.assertEqual(text.count("\n"), text.count("\r\n"), name)


class NextVersion(unittest.TestCase):
    def test_bumps(self):
        self.assertEqual(bv.next_version("0.3.0", "patch"), "0.3.1")
        self.assertEqual(bv.next_version("0.3.4", "minor"), "0.4.0")
        self.assertEqual(bv.next_version("0.3.4", "major"), "1.0.0")
        self.assertEqual(bv.next_version("1.2.0-rc1", "patch"), "1.2.1")

    def test_rejects_nonsense(self):
        with self.assertRaises(ValueError):
            bv.next_version("1.2", "patch")
        with self.assertRaises(ValueError):
            bv.next_version("1.2.3", "huge")


class RealRepository(unittest.TestCase):
    def test_every_real_version_file_matches_the_pattern_and_agrees(self):
        versions = bv.read_versions()
        self.assertTrue(all(v is not None for v in versions.values()), versions)
        self.assertEqual(len(set(versions.values())), 1, versions)


if __name__ == "__main__":
    unittest.main()
