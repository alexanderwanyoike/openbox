# 017: Resolve external media tools at the process boundary

Status: accepted.

Analysis, metadata probing, preview decoding and export transcoding share one process adapter. It uses the established `which` crate's executable search, preserving PATH precedence. macOS additionally searches `/opt/homebrew/bin` and `/usr/local/bin`, because Finder does not load an interactive shell's Homebrew PATH. There is no shell wrapper, global environment mutation or bundled FFmpeg download. If no executable is found, the normal process spawn error remains handled at the existing application boundary.

On Windows the adapter uses Rust's [CommandExt::creation_flags](https://doc.rust-lang.org/std/os/windows/process/trait.CommandExt.html#tymethod.creation_flags) with `CREATE_NO_WINDOW` for background codec processes. It does not change application windows or audio output selection.

The discovery regression launches actual executable fixtures from separate ordered directories, including fallback discovery for ffprobe. Existing real-audio analysis, decoding, metadata and export tests exercise the migrated call sites. macOS CI additionally runs audio analysis with a Finder-like restricted PATH. Baken's waveform generator decodes with Symphonia in-process and does not need an external-tool override.
