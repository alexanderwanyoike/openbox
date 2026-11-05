#!/usr/bin/env python3
import argparse
from dataclasses import dataclass
from pathlib import Path
import shutil
import sys
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class PackageError:
    message: str


def package(binary: Path, target: str, version: str, licenses: Path, output: Path) -> Path | PackageError:
    if target not in ['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc']:
        return PackageError(f'Unsupported release target: {target}')
    try:
        output.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory() as directory:
            staging = Path(directory)
            executable = staging / ('openbox.exe' if target.endswith('windows-msvc') else 'openbox')
            shutil.copyfile(binary, executable)
            executable.chmod(0o755)
            shutil.copytree(licenses, staging / 'licenses')
            for name in ['LICENSE', 'THIRD-PARTY-NOTICES.md', 'README.md']:
                shutil.copyfile(ROOT / name, staging / name)
            if target.endswith('windows-msvc'):
                archive = output / f'openbox-{target}.zip'
                with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED) as destination:
                    for item in sorted(staging.rglob('*')):
                        if item.is_file():
                            destination.write(item, arcname=item.relative_to(staging))
                return archive
            archive = output / f'openbox-{target}.tar.gz'
            with tarfile.open(archive, 'w:gz') as destination:
                for item in sorted(staging.iterdir()):
                    destination.add(item, arcname=item.name)
        return archive
    except (OSError, tarfile.TarError) as error:
        return PackageError(str(error))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--target', required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--licenses', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    arguments = parser.parse_args()
    result = package(arguments.binary, arguments.target, arguments.version, arguments.licenses, arguments.output)
    if isinstance(result, PackageError):
        print(result.message, file=sys.stderr)
        return 1
    print(result)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
