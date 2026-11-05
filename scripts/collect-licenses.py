#!/usr/bin/env python3
import argparse
from dataclasses import dataclass
from pathlib import Path
import shutil
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class CollectionError:
    message: str


def collect(vendor: Path, fftw: Path, output: Path) -> CollectionError | None:
    try:
        output.mkdir(parents=True, exist_ok=True)
        inventory = ["name\tversion\tlicense\trepository"]
        for dependency in sorted(vendor.iterdir()):
            manifest = dependency / "Cargo.toml"
            if not manifest.is_file():
                continue
            package = tomllib.loads(manifest.read_text())["package"]
            inventory.append(
                "\t".join(
                    str(package.get(field, ""))
                    for field in ["name", "version", "license", "repository"]
                )
            )
            for notice in dependency.rglob("*"):
                if notice.is_file() and notice.name.lower().startswith(
                    ("license", "licence", "copying", "notice", "copyright")
                ):
                    destination = (
                        output
                        / "cargo"
                        / dependency.name
                        / notice.relative_to(dependency)
                    )
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(notice, destination)
        for source, destination in [
            (fftw / "COPYING", "native/fftw/COPYING"),
            (
                ROOT / "crates/analysis/vendor/libkeyfinder/LICENSE",
                "native/libkeyfinder/LICENSE",
            ),
            (
                ROOT / "crates/analysis/src/grid/LICENSE.mixxx",
                "native/mixxx/LICENSE.mixxx",
            ),
            (ROOT / "resources/fonts/Manrope-LICENSE.txt", "fonts/Manrope-LICENSE.txt"),
            (
                ROOT / "crates/rekordbox/src/onelibrary/LICENSE.baken",
                "onelibrary/LICENSE.baken",
            ),
            (
                ROOT / "crates/rekordbox/src/onelibrary/LICENSE.pyrekordbox",
                "onelibrary/LICENSE.pyrekordbox",
            ),
        ]:
            target = output / destination
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
        (output / "DEPENDENCIES.tsv").write_text("\n".join(inventory) + "\n")
        return None
    except (OSError, KeyError, tomllib.TOMLDecodeError) as error:
        return CollectionError(str(error))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--vendor", type=Path, required=True)
    parser.add_argument("--fftw", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    error = collect(arguments.vendor, arguments.fftw, arguments.output)
    if error is not None:
        print(error.message, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
