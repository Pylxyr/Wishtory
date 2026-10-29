"""Unit tests for release_notes.py. Run: python3 -m unittest discover -s scripts -p "test_*.py" -v"""

import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import release_notes as rn  # noqa: E402


class PlainText(unittest.TestCase):
    def test_links_images_markup_and_urls_do_not_survive(self):
        text = "Fixed it. [Download](https://evil.example/x.exe) ![pixel](https://t.example/p.png) <b>bold</b> https://x.example/a www.y.example"
        out = rn.plain(text)
        for bad in ("http", "www.", "<", "](", "!["):
            self.assertNotIn(bad, out)
        self.assertIn("Download", out)  # the visible text is kept, the target is not

    def test_reference_links_and_autolinks(self):
        self.assertEqual(rn.plain("see [the docs][1] and <https://a.example>"), "see the docs and")

    def test_mentions_cannot_notify_anyone(self):
        out = rn.plain("thanks @octocat")
        self.assertNotIn("@octocat", out)
        self.assertEqual(out.replace("\u200b", ""), "thanks @octocat")

    def test_long_bullets_are_capped(self):
        out = rn.plain("word " * 200)
        self.assertLessEqual(len(out), 300)
        self.assertTrue(out.endswith("…"))


class TidyAiNotes(unittest.TestCase):
    def test_injected_markup_is_removed_from_every_bullet(self):
        reply = (
            "Intro that is dropped.\n"
            "### Added\n"
            "- New thing [click here](https://evil.example) <img src=x>\n"
            "  wrapped on a second line https://evil.example/2\n"
            "### Fixed\n"
            "- Crash fixed ![t](https://t.example/p.png)\n"
        )
        out = rn.tidy_ai_notes(reply)
        self.assertIsNotNone(out)
        for bad in ("http", "<img", "](", "Intro"):
            self.assertNotIn(bad, out)
        self.assertIn("### Added", out)
        self.assertIn("wrapped on a second line", out)

    def test_sections_are_ordered_deduplicated_and_empty_ones_dropped(self):
        reply = "### Fixed\n- Fixed the export crash\n### Added\n- Fixed the export crash\n- Brand new import\n"
        out = rn.tidy_ai_notes(reply)
        self.assertEqual(out.count("export crash"), 1)
        self.assertNotIn("### Changed", out)
        self.assertLess(out.index("### Added"), out.index("### Fixed") if "### Fixed" in out else len(out))

    def test_a_reply_without_bullets_is_rejected(self):
        self.assertIsNone(rn.tidy_ai_notes("Sorry, I cannot help with that."))


class DetectLogic(unittest.TestCase):
    DIFF = (
        "+++ b/src-tauri/src/main.rs\n"
        "+            commands::get_game_meta,\n"
        "+            logging::get_logs,\n"
        "-            commands::old_command,\n"
        "+++ b/src-tauri/src/pity.rs\n"
        "+pub fn history_filters(game: Game) -> u8 {\n"
    )

    def test_commands_from_every_module_are_found(self):
        seen = rn.detect_logic(self.DIFF)
        self.assertEqual(seen["commands"]["added"], {"get_game_meta", "get_logs"})
        self.assertEqual(seen["commands"]["removed"], {"old_command"})
        self.assertEqual(seen["code"]["added"], {"history_filters()"})

    def test_a_name_on_both_sides_counts_as_neither(self):
        diff = "+++ b/src-tauri/src/main.rs\n+  commands::same,\n-  commands::same,\n"
        self.assertEqual(rn.detect_logic(diff)["commands"], {"added": set(), "removed": set()})

    def test_ordinary_lines_are_not_commands(self):
        diff = "+++ b/src-tauri/src/main.rs\n+            app.manage(rules::RulesStore::open(&dir));\n+mod rules;\n"
        self.assertEqual(rn.detect_logic(diff)["commands"]["added"], set())


class PreviousTag(unittest.TestCase):
    def setUp(self):
        self.old = os.getcwd()
        self.dir = tempfile.TemporaryDirectory()
        os.chdir(self.dir.name)
        for args in (["init", "-q"], ["config", "user.name", "t"], ["config", "user.email", "t@example.com"]):
            subprocess.run(["git", *args], check=True)
        self.n = 0

    def tearDown(self):
        os.chdir(self.old)
        self.dir.cleanup()

    def release(self, tag):
        self.n += 1
        subprocess.run(["git", "commit", "-q", "--allow-empty", "-m", f"change {self.n}"], check=True)
        subprocess.run(["git", "tag", tag], check=True)

    def test_a_stable_release_skips_pre_releases(self):
        for tag in ("v1.0.0", "v1.1.0-rc1", "v1.1.0"):
            self.release(tag)
        self.assertEqual(rn.previous_tag("v1.1.0"), "v1.0.0")
        self.assertEqual(rn.previous_tag("v1.1.0-rc1"), "v1.0.0")

    def test_the_first_stable_release_falls_back_to_the_last_pre_release(self):
        for tag in ("v0.0.1-rc1", "v0.0.1-rc2", "v0.0.2"):
            self.release(tag)
        self.assertEqual(rn.previous_tag("v0.0.2"), "v0.0.1-rc2")

    def test_the_very_first_tag_has_no_predecessor(self):
        self.release("v0.0.1")
        self.assertIsNone(rn.previous_tag("v0.0.1"))

    def test_other_tags_are_ignored(self):
        for tag in ("v1.0.0", "catalog-v1", "v1.0.1"):
            self.release(tag)
        self.assertEqual(rn.previous_tag("v1.0.1"), "v1.0.0")


if __name__ == "__main__":
    unittest.main()
