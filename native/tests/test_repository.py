"""Keep the source checkout and resolved build graph native-only."""
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class NativeRepository(unittest.TestCase):
    def test_retired_build_cache_is_ignored_without_hiding_source_files(self):
        for path, ignored in [("src-tauri/target/debug/cache.json", True),
                              ("src-tauri/Cargo.toml", False)]:
            result = subprocess.run(["git", "check-ignore", "--no-index", "-q", path], cwd=ROOT)
            self.assertEqual(result.returncode, 0 if ignored else 1, path)

    def test_ci_fetches_full_graph_before_offline_audit(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        fetch = "cargo fetch --manifest-path native/Cargo.toml --locked"
        self.assertIn(fetch, workflow)
        self.assertLess(workflow.index(fetch), workflow.index("unittest discover"))
        self.assertIn("runs-on: ubuntu-24.04", workflow)

    def test_benchmark_has_no_retired_launcher_option(self):
        help_text = subprocess.check_output([
            "python3", str(ROOT / "native/benchmark.py"), "--help",
        ], text=True)
        self.assertNotIn("--tauri", help_text)
        self.assertIn("--native", help_text)

    def test_native_core_and_dependency_graph_have_no_webview_app(self):
        self.assertTrue((ROOT / "core/Cargo.toml").is_file(), "Rust core belongs in core/")
        for legacy in ["package.json", "package-lock.json", "index.html", "vite.config.ts", "src-tauri/Cargo.toml"]:
            self.assertFalse((ROOT / legacy).exists(), f"Retired app file remains: {legacy}")
        metadata = json.loads(subprocess.check_output([
            "cargo", "metadata", "--manifest-path", str(ROOT / "native/Cargo.toml"),
            "--locked", "--offline", "--format-version", "1",
        ], text=True))
        names = {package["name"] for package in metadata["packages"]}
        self.assertIn("ts4-mod-manager-native", names)
        self.assertIn("ts4-mod-manager-core", names)
        self.assertFalse(any(name.startswith(("tauri", "webkit2gtk")) or name == "wry" for name in names))
        self.assertTrue((ROOT / "native/assets/icon.png").is_file(), "Packaging owns its icon")


if __name__ == "__main__":
    unittest.main()
