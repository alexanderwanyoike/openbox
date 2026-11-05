import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]


class CollectLicensesTest(unittest.TestCase):
    def test_distribution_keeps_dependency_and_native_license_notices(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            dependency = root / 'vendor/example-1.0'
            dependency.mkdir(parents=True)
            (dependency / 'LICENSE-MIT').write_text('Example copyright and permission')
            (dependency / 'Cargo.toml').write_text('[package]\nname="example"\nversion="1.0.0"\nlicense="MIT"\n')
            fftw = root / 'fftw'
            fftw.mkdir()
            (fftw / 'COPYING').write_text('FFTW distribution terms')
            result = subprocess.run([
                sys.executable, str(ROOT / 'scripts/collect-licenses.py'),
                '--vendor', str(root / 'vendor'), '--fftw', str(fftw),
                '--output', str(root / 'licenses')
            ], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            licenses = root / 'licenses'
            self.assertEqual((licenses / 'cargo/example-1.0/LICENSE-MIT').read_text(), 'Example copyright and permission')
            self.assertEqual((licenses / 'native/fftw/COPYING').read_text(), 'FFTW distribution terms')
            self.assertTrue((licenses / 'native/libkeyfinder/LICENSE').is_file())
            self.assertTrue((licenses / 'fonts/Manrope-LICENSE.txt').is_file())
            self.assertTrue((licenses / 'onelibrary/LICENSE.baken').is_file())
            self.assertIn('example\t1.0.0\tMIT', (licenses / 'DEPENDENCIES.tsv').read_text())


if __name__ == '__main__':
    unittest.main()
