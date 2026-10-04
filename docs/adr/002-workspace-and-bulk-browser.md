# 002: Separate the workspace crates and support bulk preparation

Status: accepted. Supersedes the single-crate decision in ADR 001.

A DJ collection has hundreds of tracks. Single-track selection and repeated playlist saves make ordinary preparation slow. The desktop layout also needs to keep the collection available while preparing a device.

Use five production crates:

- `openbox-domain`: tracks, playlist folders, membership and ordering invariants. No UI or filesystem work.
- `openbox-storage`: local JSON persistence, atomic file publication and file fingerprints.
- `openbox-rekordbox`: device database parsing, preservation, backups, transfer staging, waveform generation and safe publication.
- `openbox-library`: application commands, music import and coordination of local/device operations.
- `openbox`: the Iced desktop application and Clap startup options.

`openbox-test-support` shares audio and device fixtures through dev dependencies only. Production crates do not depend on it.

Dependencies point toward the domain. The application depends on the storage and Rekordbox adapters; adapters depend on the domain. The desktop calls the application and the device adapter for background transfers. Keep concrete adapters until another implementation is needed.

A bulk command validates a selection, mutates a candidate snapshot, persists once, then publishes the saved state. Adding tracks preserves selection order and skips duplicate membership. Removing entries keeps audio and other playlist memberships. Moving a selection preserves the relative order of each selected block. Existing USB snapshot checks, backups and Undo apply to batch operations too.

Selection, filtering, sorting and viewport state belong to the desktop. Selection uses track identities rather than row positions. Shift follows the displayed order, Ctrl toggles membership, and Select all covers every filtered row, including rows outside the viewport. Changing the filter removes hidden tracks from the selection. Changing the sort preserves selected identities.

Render fixed-height track rows only around the visible viewport, with spacers retaining the full scroll extent. Keep the collection and device trees visible together. A USB playlist opens alongside the local collection, with a bulk transfer action on that source pane. Replacing one USB entry still requires exactly one source track and one device entry.

Hardware playback remains a separate validation step. File-format tests and native GUI tests cannot prove playback on an XDJ-1000.
