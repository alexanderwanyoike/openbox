# 018: Publish native archives with corresponding source

Status: accepted.

Keep the existing GitHub Actions release pattern and extend it to the four platforms verified by native CI. Build on Linux, Windows, Intel macOS and Apple Silicon macOS, then preserve those executable bytes in platform-specific archives. Linux has a root `openbox`, Windows has a root `openbox.exe` in a ZIP, and both Mac downloads contain `Openbox.app`. The package layout is shared with the updater's platform contract.

The Mac bundle follows Apple's documented `Contents/MacOS`, `Contents/Resources` and `Info.plist` structure. It contains the existing Openbox icon exported as ICNS, a versioned bundle identifier and a macOS 15 minimum matching the build hosts. The downloads are unsigned and are not notarized.

The source archive includes the exact repository revision, locked Cargo dependencies through `cargo vendor`, its source replacement configuration and the complete pinned FFTW 3.3.10 source. Cargo's offline metadata check verifies source resolution. Binary downloads include the dependency license files, native analysis notices, font license and a package inventory. System FFmpeg and ffprobe remain installation prerequisites and are not redistributed.

Pull requests build candidate downloads as workflow artifacts. Only a stable version tag whose commit is on main can create a draft GitHub release. SHA256SUMS covers all four downloads and the corresponding-source archive. Publishing the draft remains a separate release action after validation.

Patterns: [Cargo vendor](https://doc.rust-lang.org/cargo/commands/cargo-vendor.html), [Apple bundle structures](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/BundleTypes/BundleTypes.html), and the repository's existing Actions artifact and release flow.
