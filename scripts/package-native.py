#!/usr/bin/env python3
import argparse
from dataclasses import dataclass
from pathlib import Path
import shutil
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class PackageError:
    message: str


def package(binary: Path, target: str, version: str, licenses: Path, output: Path) -> Path | PackageError:
    if target != 'x86_64-unknown-linux-gnu':
        return PackageError(f'Unsupported release target: {target}')
    try:
        output.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory() as directory:
            staging = Path(directory)
            shutil.copyfile(binary, staging / 'openbox')
            (staging / 'openbox').chmod(0o755)
            shutil.copytree(licenses, staging / 'licenses')
            for name in ['LICENSE', 'THIRD-PARTY-NOTICES.md', 'README.md']:
                shutil.copyfile(ROOT / name, staging / name)
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
