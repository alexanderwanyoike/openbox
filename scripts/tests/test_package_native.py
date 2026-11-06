import pathlib
import plistlib
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[2]


class PackageNativeTest(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = pathlib.Path(directory.name)
        self.binary = self.root / "openbox"
        self.binary.write_bytes(b'#!/bin/sh\nprintf "openbox 0.1.1\\n"\n')
        self.binary.chmod(0o755)
        self.licenses = self.root / "licenses"
        self.licenses.mkdir()
        (self.licenses / "COPYING.GPL-3").write_text("Fixture distribution license")

    def package(self, target):
        return subprocess.run(
            [
                sys.executable,
                str(ROOT / "scripts/package-native.py"),
                "--binary",
                str(self.binary),
                "--target",
                target,
                "--version",
                "0.1.1",
                "--licenses",
                str(self.licenses),
                "--output",
                str(self.root / "out"),
            ],
            capture_output=True,
            text=True,
        )

    def test_linux_appimage_runs_and_contains_desktop_identity_and_notices(self):
        result = self.package("x86_64-unknown-linux-gnu")
        self.assertEqual(result.returncode, 0, result.stderr)
        image = self.root / "out/openbox-x86_64-unknown-linux-gnu.AppImage"
        self.assertTrue(image.is_file(), "Linux download must be an AppImage")
        self.assertEqual(image.read_bytes()[8:11], b"AI\x02")
        self.assertEqual(
            subprocess.check_output(
                [str(image), "--appimage-extract-and-run", "--version"], text=True
            ),
            "openbox 0.1.1\n",
        )
        subprocess.run(
            [str(image), "--appimage-extract"],
            cwd=self.root,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        unpacked = self.root / "squashfs-root"
        self.assertTrue((unpacked / "AppRun").is_file())
        self.assertTrue(
            (unpacked / "usr/share/openbox/licenses/COPYING.GPL-3").is_file()
        )
        self.assertTrue(
            (unpacked / "usr/share/openbox/THIRD-PARTY-NOTICES.md").is_file()
        )
        for name in ["musl.txt", "zstd.txt", "zlib.txt", "mimalloc.txt"]:
            self.assertTrue(
                (unpacked / "usr/share/openbox/licenses/AppImage" / name).is_file(),
                f"Missing runtime notice: {name}",
            )
        desktop = unpacked / "io.github.alexanderwanyoike.openbox.desktop"
        self.assertIn("Icon=io.github.alexanderwanyoike.openbox", desktop.read_text())
        self.assertTrue(
            (unpacked / "io.github.alexanderwanyoike.openbox.svg").is_file()
        )

    def test_linux_packages_the_license_inventory_from_the_source_job(self):
        notices = self.licenses / "AppImage"
        notices.mkdir()
        (notices / "source-inventory.txt").write_text("Preserve source-stage notices")
        result = self.package("x86_64-unknown-linux-gnu")
        self.assertEqual(result.returncode, 0, result.stderr)
        image = self.root / "out/openbox-x86_64-unknown-linux-gnu.AppImage"
        subprocess.run(
            [str(image), "--appimage-extract"],
            cwd=self.root,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        packaged = self.root / "squashfs-root/usr/share/openbox/licenses/AppImage"
        self.assertEqual(
            (packaged / "source-inventory.txt").read_text(),
            "Preserve source-stage notices",
        )
        self.assertTrue((packaged / "AppImage-runtime.txt").is_file())

    def test_old_linux_installs_keep_a_compatible_update_bridge(self):
        result = self.package("x86_64-unknown-linux-gnu")
        self.assertEqual(result.returncode, 0, result.stderr)
        bridge = self.root / "out/openbox-x86_64-unknown-linux-gnu.tar.gz"
        self.assertTrue(
            bridge.is_file(), "The 0.1.1 updater still requests the legacy asset"
        )
        with tarfile.open(bridge) as archive:
            archive.extractall(self.root / "legacy", filter="data")
        self.assertEqual(
            subprocess.check_output(
                [str(self.root / "legacy/openbox"), "--version"], text=True
            ),
            "openbox 0.1.1\n",
        )

    def test_windows_zip_contains_the_executable_and_distribution_notices(self):
        result = self.package("x86_64-pc-windows-msvc")
        self.assertEqual(result.returncode, 0, result.stderr)
        with zipfile.ZipFile(
            self.root / "out/openbox-x86_64-pc-windows-msvc.zip"
        ) as archive:
            self.assertEqual(archive.read("openbox.exe"), self.binary.read_bytes())
            self.assertIn("licenses/COPYING.GPL-3", archive.namelist())
            self.assertIn("THIRD-PARTY-NOTICES.md", archive.namelist())

    def test_mac_archives_contain_a_versioned_application_bundle_and_icon(self):
        for target in ["aarch64-apple-darwin", "x86_64-apple-darwin"]:
            with self.subTest(target=target):
                result = self.package(target)
                self.assertEqual(result.returncode, 0, result.stderr)
                with tarfile.open(
                    self.root / f"out/openbox-{target}.tar.gz"
                ) as archive:
                    archive.extractall(self.root / target, filter="data")
                contents = self.root / target / "Openbox.app/Contents"
                info = plistlib.loads((contents / "Info.plist").read_bytes())
                self.assertEqual(info["CFBundleExecutable"], "openbox")
                self.assertEqual(info["CFBundleShortVersionString"], "0.1.1")
                self.assertEqual(
                    info["CFBundleIdentifier"], "io.github.alexanderwanyoike.openbox"
                )
                self.assertEqual(
                    (contents / "MacOS/openbox").read_bytes(), self.binary.read_bytes()
                )
                icon = contents / "Resources" / info["CFBundleIconFile"]
                self.assertTrue(icon.read_bytes().startswith(b"icns"))
                self.assertTrue(
                    (contents / "Resources/licenses/COPYING.GPL-3").is_file()
                )


if __name__ == "__main__":
    unittest.main()
