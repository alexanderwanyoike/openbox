# Workspace validation

Validated on Linux with Rust 1.96.1 and Iced 0.14. Commands use Cargo directly.

| Check | Result |
|---|---|
| `cargo test --workspace --no-default-features` | Core crate tests |
| `cargo test --workspace --all-targets` | 71 tests passed; 2 manual timing harnesses ignored, including audio mixing, waveform seeking, export progress, selection, bulk commands and startup parsing |
| `cargo check --workspace --all-targets` | Passed for desktop and all targets |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed with warnings treated as errors |
| `cargo fmt --all -- --check` | Passed |
| Native smoke harness (`scripts/smoke.mjs`) | Native Iced workflow passed under Xvfb with release and development executables |
| Audio smoke harness (`scripts/players-smoke.mjs`) | Native two-deck audio and export workflow passed with an isolated PulseAudio/PipeWire test sink |
| `cargo build` | Passed with optimized workspace code and dependencies |
| `cargo build --release` | Passed; stripped executable approximately 18 MB |
| Native analysis harness (`scripts/analysis-smoke.mjs`) | Bulk reanalysis, ID3 tags, visible artwork, grid and USB artwork/analysis export passed |
| XDJ-1000 playback | User confirmed playlist visibility; new grid, BPM/key and artwork behavior pending hardware |

The tests were developed incrementally through observed failing behavior and passing implementations for import identity, playlist sharing, validation, ordering, deletion, persistence, real audio import, device export, USB edits, and save failure recovery.

The export integration test uses generated WAV audio, invokes FFmpeg, and independently reads the resulting `export.pdb`, DAT, and EXT files with `rekordcrate`. It checks track and playlist counts, shared audio, playlist membership, and parseable analysis sections. Further checks cover relocation to a new mount path, existing Rekordbox USB merging, sparse track and folder IDs, source reuse, track replacement, database backups, external-change conflicts, writer lock release, malformed/cyclic pages, symbolic links, failed transfers, and failed playlist saves. A real Rekordbox database fixture is included under `tests/fixtures/` and verifies that original track pages remain unchanged.

The native smoke test uses actual mouse and keyboard input in the optimized Iced executable. It starts with a real Rekordbox database fixture, generated audio and a 510-track library in an isolated folder. It adds all 500 filtered tracks with Ctrl+A and one playlist action, checks non-adjacent Ctrl-toggle selection, scrolls through the large playlist, removes a Shift range, transfers a sorted batch to an existing USB playlist, replaces a device song, removes a batch, deletes a playlist with Undo, imports copied audio, and creates/imports a nested folder tree. It also creates a populated playlist from selected tracks and types its new name into the automatically focused field. Original audio, analysis data and database pages are checked byte for byte.

The application tests separately verify a 500-track bulk save, duplicate membership handling, preserved selection order, and rejection of an invalid batch without publishing partial changes. The domain tests exercise selected-block reordering. Browser tests verify filtering, sorting, selection across non-visible rows and bounded viewport rendering. The Rekordbox batch integration test transfers three real audio files and retains the original device records and files.

Audio tests decode real generated WAV audio and verify waveform envelopes, two simultaneous decks, crossfader endpoints, paused transport, seeking, pitch, temporary cue points, EQ, output limiting and end-of-track handling through the mixer used by the output callback. UI tests cover waveform seek coordinates and rejection of stale deck-load results. Progress tests verify monotonic percentages, successful reopen before 100%, and clearing a failed export from the status bar.

The player smoke harness loads two generated 90-second tracks through native mouse input. It routes output to its own null sink, records the monitor through `parec`, and checks the recorded signal's frequencies and level. It verifies both decks playing, each crossfader endpoint, channel/master mute, pitch change, overview seeking into a different frequency section, detailed waveform drag, cue return, pause, hiding/showing players without interrupting sound, and export. Local library data and source audio remain byte for byte. The temporary sink is removed afterwards; the desktop's default output is not changed. The harness requires `pactl` and `parec` in addition to the native GUI smoke dependencies.

The earlier demo window used approximately 38 MiB resident memory in this environment. This is one observation with ten tracks, not a benchmark for large libraries.

No physical USB was formatted or written during validation. Export fixtures were generated in local test directories. See [the hardware checklist](HARDWARE-VALIDATION.md) before claiming hardware readiness.

## Analysis and artwork checks

The new analysis tests feed untagged generated rhythmic/harmonic audio into the actual aubio/libkeyfinder engines, check tempo within 0.5 BPM of the WAV fixture, detected C major, increasing beat timestamps and timing near known pulses, and ensure silence/corrupt inputs do not receive fabricated analysis. Import integration confirms the source bytes are unchanged. Reanalysis refreshes tags while preserving track IDs, membership, and loaded preview positions.

Export tests use rekordcrate to independently parse PQTZ, the key/BPM records, metadata and artwork rows. The MP3 priming test compares an exported grid point to the peak of decoded exported audio with a one-millisecond tolerance. Cover export checks embedded ID3 art, 240x240 device art, ID allocation alongside existing artwork, reuse and byte preservation of original assets. USB import tests unplug the temporary device and verify local artwork/audio/grid remain available. Legacy tracks are analyzed during export without altering the local library.

The native analysis harness uses a real tagged MP3 with front-cover art, clicks Analyze, loads the analyzed track in a deck, checks the visible cover's pixels, exports its playlist, and checks persisted analysis/artwork and unchanged source bytes. The audio harness drags each channel fader, checks independent muting through recorded output, resets the fader with Ctrl-click, and checks the trim knob separately. It also checks the crossfader, pitch, cue and seek controls. A mixer regression test verifies trim and fader multiplication, a live pre-fader meter, continued playback with a closed fader, and isolation between channels.

Automatic beat numbering assumes 4/4 from the first detected beat. Musical downbeat identification is not implemented. These checks prove file contents and app behavior, not player sync/quantize compatibility. No physical USB was written during automated validation.

## Waveform and interaction polish

The rendered-pixel regression checks blue bass ahead of the playhead and a warm waveform behind it using the real software renderer. The envelope tests check frequency-band dominance, higher-resolution detail, silence and invalid time bounds. Position tests check interpolation between snapshots, pitch, pause and track end. A knob test reproduces consecutive drags being mistaken for double-click reset. A native layout test verifies the library retains a useful viewport when the player panel is toggled.

`OPENBOX_CHECK_MOTION=1` adds a 60 fps X11 capture to the player harness. It compares consecutive waveform crops, excluding the cursor, and requires at least 45 distinct updates per second. In this environment the previous release produced 23.1 fps; the GPU development build produced 53 distinct frames/sec on the Intel UHD desktop. Xvfb software rendering measured about 23 fps with the detailed rulers. These are local fixture measurements rather than a guarantee for every GPU, display, CPU or library.

Three ignored timing harnesses can be run with `cargo test --release profile_ -- --ignored --nocapture`. They measure software waveform rasterization and construction/layout of a cached, filtered/sorted 5,000-track browser. Warm browser construction/layout measured about 0.08 ms at p95 after caching; previously browser construction alone measured about 3.8 ms. The overview cache and bounded envelope queries prevent full-song scans during animation. See ADR 005 for the rendering and cache rules.

The development profile optimizes the software rasterizer without disabling application or USB invariant checks. Its detailed waveform rasterization measured about 2.0 ms at p95, versus 4.4 ms with the previous renderer settings.

## Appearance and panels

The native settings journey exercises both modes, eight materials, six accents and ten waveform palettes, preference persistence across restart, restoring defaults and retaining the selected browser row after closing Settings. It checks that library and audio bytes are unchanged. Palette tests exercise 96 combinations for text contrast, accent saturation and persistence. A native layout test checks separate inset panels for the two decks and mixer. The default background/panel/chrome test uses the exact neutral Showbiz surface colours. Screenshots capture the actual native window.
