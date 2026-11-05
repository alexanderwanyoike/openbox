# 010: OneLibrary and paired device publication

Status: Accepted

## Context

Traditional players read `PIONEER/rekordbox/export.pdb`. OneLibrary players read the encrypted `exportLibrary.db` in the same directory. Both must reference the same track identities, audio and analysis paths. Existing Rekordbox data must survive edits.

## Decision

`openbox-rekordbox` owns both formats. The domain library has no database or encryption dependencies. When only OneLibrary exists, the adapter constructs a traditional database model in memory so existing domain operations can be reused. Reads copy encrypted data and its WAL into a host temporary directory; they never open the actual USB database through SQLite or modify USB files.

The schema and key encoding follow [Baken commit 8446116b](https://github.com/M-Igashi/baken/tree/8446116b51dba57370c37ff3bcc1efd69ab9ab40). Their MIT notices and pyrekordbox's MIT notice are included beside the adapter. Encryption and page authentication use SQLCipher through rusqlite with explicit [SQLCipher 4 compatibility](https://www.zetetic.net/sqlcipher/sqlcipher-api/#cipher_compatibility). Openbox does not implement encryption primitives.

New exports write both databases. Updates retain the existing OneLibrary database as the base, including its schema, unknown columns, unknown tables, cues, history and hot-cue banks. Existing content records remain intact. New content and lookup records use explicit insert columns. Playlist edits preserve surviving membership rows and their unknown fields. Library identities and the original playlist tree must agree between formats before an update. A conflicting tree or unsupported schema is refused rather than rebuilt destructively. Nonempty device WAL files are refused before saving or recovering, because their committed changes may not appear in the main-file fingerprint.

Writer guards explicitly release their shared file lock on drop. A descriptor duplicated during concurrent process creation must not prolong a completed write; the guard uses [`File::unlock`](https://doc.rust-lang.org/std/fs/struct.File.html#method.unlock) before its handle closes.

Publication follows SQLite's [rollback journal pattern](https://www.sqlite.org/atomiccommit.html): retain the previous files, record their old and proposed fingerprints in `.openbox/pending.json`, publish the two databases and Openbox cache, flush, then remove the marker. Backups remain under `.openbox/backups/`. An interrupted marker prevents read-only opening. Explicit `recover` or the next export acquires the writer lock, validates every current file and backup, and restores the previous set. Conflicting newer files or damaged backups prevent automatic restoration. Uncertain publication retains any newly exported audio.

## Limits

This backend change does not add the desktop's OneLibrary discovery or recovery action. Those integrations are required before release. Device-format tests do not establish physical player compatibility. OneLibrary playback, analysis and artwork still require hardware validation.

Recovery covers interrupted process publication. It cannot guarantee drive-controller or filesystem behaviour during power loss or unplugging. Unix directory entries are flushed; Windows currently has regular-file flushing without an equivalent directory flush through `std::fs`. Do not delete a device WAL or recovery marker to bypass a conflict.
