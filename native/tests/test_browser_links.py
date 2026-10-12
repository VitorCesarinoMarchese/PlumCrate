"""Guard desktop browser dispatch in the resolved shipping dependency graph."""
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


class DesktopBrowserLinks(unittest.TestCase):
    def test_shipping_backend_can_open_browser_links(self):
        result = subprocess.run([
            "cargo", "metadata", "--manifest-path", "native/Cargo.toml",
            "--locked", "--format-version", "1", "--offline",
        ], cwd=ROOT, check=True, capture_output=True, text=True)
        metadata = json.loads(result.stdout)
        backend_ids = {package["id"] for package in metadata["packages"]
                       if package["name"] == "egui-winit"}
        backends = [node for node in metadata["resolve"]["nodes"]
                    if node["id"] in backend_ids]
        self.assertTrue(backends, "shipping desktop backend must be resolved")
        for backend in backends:
            self.assertIn("webbrowser", backend["features"],
                          "desktop backend discards OpenUrl without browser dispatch support")


    def test_shipping_wayland_runtime_uses_matched_occlusion_fixes(self):
        metadata = json.loads(subprocess.check_output([
            "cargo", "metadata", "--manifest-path", "native/Cargo.toml",
            "--locked", "--format-version", "1", "--offline",
        ], cwd=ROOT, text=True))
        gui = {"ecolor", "eframe", "egui", "egui-wgpu", "egui-winit", "egui_extras",
               "egui_glow", "emath", "epaint", "epaint_default_fonts"}
        packages = [package for package in metadata["packages"] if package["name"] in gui]
        self.assertEqual({package["name"] for package in packages}, gui)
        self.assertEqual(len(packages), len(gui), "GUI crates must share one runtime")
        for package in packages:
            self.assertTrue(package["source"].endswith("#ba6790fe7cf46e58e8d27ce1524cbfdee745e938"), package["name"])
        windows = [package for package in metadata["packages"] if package["name"] == "winit"]
        self.assertEqual(len(windows), 1)
        self.assertTrue(windows[0]["source"].endswith("#ed7caa9023f10b397f5b6ec8284a840cbd8a6f65"))


if __name__ == "__main__":
    unittest.main()
