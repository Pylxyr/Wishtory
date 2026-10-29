"""fetch-published.sh must treat only a missing release as "nothing published"."""

import os
import stat
import subprocess
import tempfile
import unittest

SCRIPT = os.path.join(os.path.dirname(__file__), "fetch-published.sh")

STUB = """#!/usr/bin/env bash
case "$1" in
  api)
    case "$STUB_MODE" in
      missing) echo "gh: Not Found (HTTP 404)" >&2; exit 1;;
      broken)  echo "error connecting to api.github.com" >&2; exit 1;;
      ok)      printf 'catalog.envelope.json\\nbanners.envelope.json\\nbanners.json\\n';;
    esac;;
  release)
    while [ $# -gt 0 ]; do case "$1" in --pattern) pat="$2";; --dir) dir="$2";; esac; shift; done
    echo '{"payload":"e30=","signature":"x"}' > "$dir/$pat";;
esac
"""


class FetchPublished(unittest.TestCase):
    def run_script(self, mode):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        stub = os.path.join(tmp.name, "gh")
        with open(stub, "w") as f:
            f.write(STUB)
        os.chmod(stub, os.stat(stub).st_mode | stat.S_IXUSR)
        out = os.path.join(tmp.name, "published")
        env = {**os.environ, "PATH": f"{tmp.name}:{os.environ['PATH']}", "STUB_MODE": mode, "GITHUB_REPOSITORY": "o/r"}
        run = subprocess.run(["bash", SCRIPT, out], env=env, capture_output=True, text=True)
        files = sorted(os.listdir(out)) if os.path.isdir(out) else []
        return run, files

    def test_a_missing_release_means_nothing_is_published(self):
        run, files = self.run_script("missing")
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual(files, ["assets.txt"])

    def test_a_failure_is_not_mistaken_for_a_first_publish(self):
        run, _ = self.run_script("broken")
        self.assertNotEqual(run.returncode, 0)
        self.assertIn("Could not read the catalog release", run.stderr)

    def test_only_feeds_that_exist_are_downloaded(self):
        run, files = self.run_script("ok")
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertEqual(files, ["assets.txt", "banners.envelope.json", "catalog.envelope.json"])


if __name__ == "__main__":
    unittest.main()
