#!/usr/bin/env python3
"""Fetch the official, checksum-pinned AppImage tools and runtime sources."""

import argparse
from dataclasses import dataclass
import hashlib
from pathlib import Path
import shutil
import tarfile
import urllib.error
import urllib.request

RUNTIME_COMMIT = "dd6cebedcbddde9c82f89b011e8e1d40b6e43868"
DOWNLOADS = [
    (
        "appimagetool",
        "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage",
        "ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0",
    ),
    (
        "runtime-x86_64",
        "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64",
        "2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d",
    ),
    (
        "source/type2-runtime.tar.gz",
        f"https://codeload.github.com/AppImage/type2-runtime/tar.gz/{RUNTIME_COMMIT}",
        "f5fec23be76e50e2445ed2d018bac49b367490fc483c62c4637e99ec705d27ba",
    ),
    (
        "source/fuse-3.15.0.tar.xz",
        "https://github.com/libfuse/libfuse/releases/download/fuse-3.15.0/fuse-3.15.0.tar.xz",
        "70589cfd5e1cff7ccd6ac91c86c01be340b227285c5e200baa284e401eea2ca0",
    ),
    (
        "source/squashfuse-0.5.2.tar.gz",
        "https://codeload.github.com/vasi/squashfuse/tar.gz/0.5.2",
        "db0238c5981dabbd80ee09ae15387f390091668ca060a7bc38047912491443d3",
    ),
]


@dataclass(frozen=True)
class PreparationError:
    message: str


def prepare(output: Path) -> Path | PreparationError:
    try:
        for name, url, expected in DOWNLOADS:
            destination = output / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            if not destination.exists():
                with urllib.request.urlopen(url, timeout=120) as response:
                    data = response.read()
                if hashlib.sha256(data).hexdigest() != expected:
                    return PreparationError(f"Checksum mismatch for {name}")
                destination.write_bytes(data)
            if hashlib.sha256(destination.read_bytes()).hexdigest() != expected:
                return PreparationError(f"Checksum mismatch for {name}")
        (output / "appimagetool").chmod(0o755)
        notices = output / "licenses"
        notices.mkdir(exist_ok=True)
        shutil.copytree(
            Path(__file__).resolve().parents[1] / "resources/linux/appimage-licenses",
            notices,
            dirs_exist_ok=True,
        )
        for archive, member, name in [
            (
                "type2-runtime.tar.gz",
                f"type2-runtime-{RUNTIME_COMMIT}/LICENSE",
                "AppImage-runtime.txt",
            ),
            ("fuse-3.15.0.tar.xz", "fuse-3.15.0/LGPL2.txt", "libfuse-LGPL2.txt"),
            ("squashfuse-0.5.2.tar.gz", "squashfuse-0.5.2/LICENSE", "squashfuse.txt"),
        ]:
            with tarfile.open(output / "source" / archive) as source:
                license_file = source.extractfile(member)
                if license_file is None:
                    return PreparationError(f"Missing license in {archive}")
                with license_file, (notices / name).open("wb") as destination:
                    shutil.copyfileobj(license_file, destination)
        return output
    except (OSError, urllib.error.URLError, tarfile.TarError, KeyError) as error:
        return PreparationError(str(error))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path(".native/appimage"))
    arguments = parser.parse_args()
    result = prepare(arguments.output)
    if isinstance(result, PreparationError):
        parser.exit(1, f"{result.message}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
