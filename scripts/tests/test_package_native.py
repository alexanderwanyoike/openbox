import pathlib
import subprocess
import sys
import tarfile
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]


class PackageNativeTest(unittest.TestCase):
    def test_linux_archive_preserves_executable_and_distribution_notices(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            binary = root / 'openbox'
            binary.write_bytes(b'#!/bin/sh\nprintf "openbox 0.1.1\\n"\n')
            binary.chmod(0o755)
            licenses = root / 'licenses'
            licenses.mkdir()
            (licenses / 'COPYING.GPL-3').write_text('Fixture distribution license')
            result = subprocess.run([
                sys.executable, str(ROOT / 'scripts/package-native.py'),
                '--binary', str(binary), '--target', 'x86_64-unknown-linux-gnu',
                '--version', '0.1.1', '--licenses', str(licenses), '--output', str(root / 'out')
            ], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            archive = root / 'out/openbox-x86_64-unknown-linux-gnu.tar.gz'
            with tarfile.open(archive) as package:
                package.extractall(root / 'unpacked', filter='data')
            unpacked = root / 'unpacked'
            self.assertTrue((unpacked / 'licenses/COPYING.GPL-3').is_file())
            self.assertTrue((unpacked / 'THIRD-PARTY-NOTICES.md').is_file())
            self.assertEqual(subprocess.check_output([str(unpacked / 'openbox'), '--version'], text=True), 'openbox 0.1.1\n')


if __name__ == '__main__':
    unittest.main()
