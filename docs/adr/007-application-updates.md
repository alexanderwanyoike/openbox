# Linux application updates

The desktop footer checks this repository's latest published stable GitHub release. Clicking an available version downloads, verifies, replaces and relaunches the executable. Playback, decoding, imports and device work block installation. Inputs remain locked during replacement and restart.

The adapter uses MIT-licensed `self_update` 1.3.3 and `self-replace`, rather than implementing archive extraction or executable replacement. `semver` handles version ordering. The release version is pinned between discovery and installation. Both GitHub's asset digest and the digest captured at discovery are checked. The extracted executable must successfully report the expected version before replacement. Checksums verify integrity; this pipeline does not claim independent publisher signatures.

Release artifacts are Linux x86_64 tarballs containing `openbox` at the archive root. Install the binary in a directory writable by the user. FFmpeg, ALSA and FFTW3 remain system dependencies. Package-manager-owned installations should be updated through their package manager.

Only stable tags matching the workspace version and contained in main can build releases. Tests, formatting and linting precede packaging. A draft GitHub Release includes the executable, SHA256SUMS and the repository plus vendored Cargo dependencies as corresponding source. Publishing the draft makes it visible to the updater. Feature branches and dev do not publish releases.

The library and command-line arguments survive restart. Deck playback is intentionally paused before installation and preview sessions are not persisted. A failed restart offers a retry using the captured replacement path. No physical USB is involved in automated update verification.

Tests use a local HTTP server, real tarballs, a disposable installed file and an actual child process. They cover stable ordering, release discovery, checksum failure, replacement and argument-preserving restart. Debug builds alone accept `OPENBOX_UPDATE_TEST_API=http://127.0.0.1:PORT` for isolated native GUI verification. Production builds always use GitHub.

References:
- https://docs.rs/self_update/1.3.3/self_update/
- https://github.com/jaemk/self_update
- Showbiz's release/update separation and active-work guards in `../showbiz/frontend/docs/automatic-updates.md`, MIT licensed by Alexander Wanyoike.
