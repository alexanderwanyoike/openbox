# 019: Match updates to native release packages

Status: accepted.

Select the exact current target's download, retaining stable-version checks and the published SHA256 digest requirement. Supported targets are Linux x86_64, Windows x86_64 and both Mac architectures. Unsupported targets return an explicit error. Windows archives contain `openbox.exe` in a ZIP; Linux contains `openbox` in a tarball.

Use the existing `self_update` 1.3.3 package and its documented complete-bundle swap for macOS. Downloads contain `Openbox.app`. The installer derives the destination bundle from the executable path and verifies the staged `Contents/MacOS/openbox --version` before replacing the entire bundle. This keeps metadata and resources aligned with the executable. Standalone Mac executables outside a bundle return an explicit layout error. Restart preserves the current application arguments.

A native Apple Silicon run first demonstrated the old Linux-only selection rejecting the correct Mac asset as InvalidRelease. The native archive test uses a compiled executable, checks full bundle resource replacement on macOS, verifies checksum rejection preserves the installed file, and exercises argument-preserving restart. Every install target is disposable and served by a loopback fixture.

Pattern: the pinned upstream self_update crate's bundle-install and checksum APIs. The adapter does not implement archive extraction or transactional replacement itself.

Linux now selects the exact `.AppImage` asset. While running inside an AppImage, the install destination is the validated outer `APPIMAGE` file; the executable must actually reside inside `APPDIR` before those environment variables are trusted. Ordinary executable launches retain their own install path. Validation and restart clear the old runtime environment and enable extract-and-run, so the update works without FUSE. The existing checksum-before-replacement and disposable-install verification remain in force. The application keeps the user's original arguments when restarting.
