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

    def test_linux_archive_preserves_executable_and_distribution_notices(self):
        result = self.package("x86_64-unknown-linux-gnu")
        self.assertEqual(result.returncode, 0, result.stderr)
        archive = self.root / "out/openbox-x86_64-unknown-linux-gnu.tar.gz"
        with tarfile.open(archive) as package:
            package.extractall(self.root / "unpacked", filter="data")
        unpacked = self.root / "unpacked"
        self.assertTrue((unpacked / "licenses/COPYING.GPL-3").is_file())
        self.assertTrue((unpacked / "THIRD-PARTY-NOTICES.md").is_file())
        desktop = (
            unpacked / "share/applications/io.github.alexanderwanyoike.openbox.desktop"
        )
        self.assertIn("Icon=io.github.alexanderwanyoike.openbox", desktop.read_text())
        self.assertTrue(
            (
                unpacked
                / "share/icons/hicolor/scalable/apps/io.github.alexanderwanyoike.openbox.svg"
            ).is_file()
        )
        self.assertEqual(
            subprocess.check_output(
                [str(unpacked / "openbox"), "--version"], text=True
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
