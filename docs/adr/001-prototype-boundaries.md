# 001: Keep the native prototype small

Status: the single-crate decision is superseded by [ADR 002](002-workspace-and-bulk-browser.md). Device preservation and export constraints remain in effect.

The question is whether library management, playlist preparation, real device export, and separate USB playlist editing form a useful small application.

## Decision

Use one Rust crate with a pure domain model, an application controller, concrete infrastructure adapters, and an Iced interface. Use the same library model and editing commands for the local collection and the USB snapshot.

`domain` owns playlist invariants and track identity. `application::Workspace` handles edits by preparing a new state, persisting it, then publishing it in memory. It delegates audio import, storage, and device export to their respective modules. The UI coordinates user messages and runs import/export work off its event thread.

The USB module adapts the MIT-licensed Bake'n Deck exporter, pinned to a Git revision. `usb::database` reads bounded page chains and raw rows; it retains original database pages rather than regenerating existing track records. Playlist writes append new playlist tables and redirect the header. Adding tracks appends metadata with collision-free IDs and preserves the existing folder hierarchy and unrelated tables. `usb::transfer` stages audio, resolves identities, and merges playlist membership. `usb::write` handles locking, backups, conflicts, publication, and rollback. `export.pdb` is authoritative; the JSON cache is trusted only when its database fingerprint matches. Tests use the independent `rekordcrate` parser as a format oracle. Neither library proves XDJ-1000 playback compatibility by itself.

Store the prototype session as JSON under `.prototype/`. This avoids schema migrations while evaluating the workflows. Keep filesystem persistence out of the domain so SQLite can be introduced if the validated requirements need it.

## Patterns and constraints

- Iced's message/update/view structure is the UI controller.
- `Edit` is a small command vocabulary reused for local and device playlists.
- Import and USB export are adapters around external tools and formats.
- The domain model owns the information required to enforce its rules, following GRASP Information Expert.
- Modules each have a reason to change. Concrete adapters are sufficient here; add traits when there is a second implementation or an actual testing need.

SOLID does not require a trait or service for every struct. Avoid generic repositories, event buses, dependency injection containers, plugins, and speculative analysis engines.

## Limits

Export encodes all selected audio to stereo 44.1 kHz, 320 kbps CBR MP3. It generates DAT and EXT waveforms. It does not invent beatgrids or cues from BPM tags. Existing Rekordbox sticks are read and edited in place. Playlist folders and leaves share one node ID namespace, with parent references and sibling ordering. The UI renders that tree and creates nodes in the selected folder. Transfers recreate ancestor folders and match playlists by path. Moving or deleting folders, original-format export, automatic BPM/key detection, local cue editing, and playback are outside this prototype.

Unchanged transfer sources reuse their encoded audio after checking source and device fingerprints. Replaced songs remain in the USB collection. Original USB files are never eligible for cleanup. Device playlist deletion never deletes audio. Normal write failures restore the previous database and remove only files created by the failing transfer; the two-file metadata commit is not a journaled transaction against sudden device removal or power loss.

The page layout is documented by [Deep Symmetry's export analysis](https://djl-analysis.deepsymmetry.org/rekordbox-export-analysis/exports.html). File-format tests include a real Rekordbox database fixture from rekordcrate, with upstream provenance and license under `tests/fixtures/`.

## Verdict

Pending user review and a physical XDJ-1000 playback test. Keep the boundaries that prove useful; replace or remove prototype-specific persistence and demo infrastructure before a production release.
