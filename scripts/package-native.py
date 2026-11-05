#!/usr/bin/env python3
import argparse
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
import shutil
import plistlib
import sys
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class PackageError:
    message: str


def package(
    binary: Path,
    target: str,
    version: str,
    licenses: Path,
    output: Path,
    appimage_tools: Path = ROOT / ".native/appimage",
) -> Path | PackageError:
    if target not in [
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
    ]:
        return PackageError(f"Unsupported release target: {target}")
    try:
        output.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory() as directory:
            staging = Path(directory)
            is_mac = target.endswith("apple-darwin")
            contents = staging / "Openbox.app/Contents"
            if is_mac:
                (contents / "MacOS").mkdir(parents=True)
                (contents / "Resources").mkdir()
                info = {
                    "CFBundleName": "Openbox",
                    "CFBundleIdentifier": "io.github.alexanderwanyoike.openbox",
                    "CFBundleExecutable": "openbox",
                    "CFBundlePackageType": "APPL",
                    "CFBundleShortVersionString": version,
                    "CFBundleVersion": version,
                    "CFBundleIconFile": "openbox.icns",
                    "LSMinimumSystemVersion": "15.0",
                    "NSHighResolutionCapable": True,
                }
                (contents / "Info.plist").write_bytes(plistlib.dumps(info))
                shutil.copyfile(
                    ROOT / "resources/icons/openbox.icns",
                    contents / "Resources/openbox.icns",
                )
            executable = (
                contents / "MacOS/openbox"
                if is_mac
                else staging
                / ("openbox.exe" if target.endswith("windows-msvc") else "openbox")
            )
            shutil.copyfile(binary, executable)
            executable.chmod(0o755)
            if target == "x86_64-unknown-linux-gnu":
                identity = "io.github.alexanderwanyoike.openbox"
                desktop = staging / "share/applications"
                desktop.mkdir(parents=True)
                shutil.copyfile(
                    ROOT / f"resources/linux/{identity}.desktop",
                    desktop / f"{identity}.desktop",
                )
                for size, extension in [
                    ("scalable", "svg"),
                    ("256x256", "png"),
                    ("512x512", "png"),
                ]:
                    icons = staging / f"share/icons/hicolor/{size}/apps"
                    icons.mkdir(parents=True)
                    source = (
                        "openbox.svg"
                        if extension == "svg"
                        else f"openbox-{size.split('x')[0]}.png"
                    )
                    shutil.copyfile(
                        ROOT / "resources/icons" / source,
                        icons / f"{identity}.{extension}",
                    )
            legal = contents / "Resources" if is_mac else staging
            shutil.copytree(licenses, legal / "licenses")
            for name in ["LICENSE", "THIRD-PARTY-NOTICES.md", "README.md"]:
                shutil.copyfile(ROOT / name, legal / name)
            if target.endswith("windows-msvc"):
                archive = output / f"openbox-{target}.zip"
                with zipfile.ZipFile(
                    archive, "w", compression=zipfile.ZIP_DEFLATED
                ) as destination:
                    for item in sorted(staging.rglob("*")):
                        if item.is_file():
                            destination.write(item, arcname=item.relative_to(staging))
                return archive
            if target == "x86_64-unknown-linux-gnu":
                # Version 0.1.1 requests this filename before it knows about AppImages.
                with tarfile.open(
                    output / f"openbox-{target}.tar.gz", "w:gz"
                ) as bridge:
                    for item in sorted(staging.iterdir()):
                        bridge.add(item, arcname=item.name)
                appdir = staging / "Openbox.AppDir"
                (appdir / "usr/bin").mkdir(parents=True)
                shutil.move(staging / "openbox", appdir / "usr/bin/openbox")
                shutil.move(staging / "share", appdir / "usr/share")
                legal = appdir / "usr/share/openbox"
                legal.mkdir()
                for name in [
                    "licenses",
                    "LICENSE",
                    "THIRD-PARTY-NOTICES.md",
                    "README.md",
                ]:
                    shutil.move(staging / name, legal / name)
                shutil.copytree(
                    appimage_tools / "licenses", legal / "licenses/AppImage"
                )
                (appdir / "AppRun").symlink_to("usr/bin/openbox")
                (appdir / f"{identity}.desktop").symlink_to(
                    f"usr/share/applications/{identity}.desktop"
                )
                (appdir / f"{identity}.svg").symlink_to(
                    f"usr/share/icons/hicolor/scalable/apps/{identity}.svg"
                )
                (appdir / ".DirIcon").symlink_to(f"{identity}.svg")
                image = (output / f"openbox-{target}.AppImage").resolve()
                subprocess.run(
                    [
                        str(appimage_tools.resolve() / "appimagetool"),
                        "--appimage-extract-and-run",
                        "--no-appstream",
                        "--runtime-file",
                        str(appimage_tools.resolve() / "runtime-x86_64"),
                        str(appdir),
                        str(image),
                    ],
                    env={**os.environ, "ARCH": "x86_64", "VERSION": version},
                    check=True,
                    capture_output=True,
                    text=True,
                )
                image.chmod(0o755)
                return image
            archive = output / f"openbox-{target}.tar.gz"
            with tarfile.open(archive, "w:gz") as destination:
                for item in sorted(staging.iterdir()):
                    destination.add(item, arcname=item.name)
        return archive
    except subprocess.CalledProcessError as error:
        return PackageError(error.stderr or str(error))
    except (OSError, tarfile.TarError) as error:
        return PackageError(str(error))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--licenses", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--appimage-tools",
        type=Path,
        default=Path(
            os.environ.get("OPENBOX_APPIMAGE_TOOLS", ROOT / ".native/appimage")
        ),
    )
    arguments = parser.parse_args()
    result = package(
        arguments.binary,
        arguments.target,
        arguments.version,
        arguments.licenses,
        arguments.output,
        arguments.appimage_tools,
    )
    if isinstance(result, PackageError):
        print(result.message, file=sys.stderr)
        return 1
    print(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
