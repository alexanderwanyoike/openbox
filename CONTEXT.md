# Openbox prototype

Openbox organises local music files and ordered playlists, and exports a traditional Rekordbox Device Library for XDJ-1000 players. This is a native Rust application using Iced, with two preview decks and a mixer for auditioning transitions.

- A **track** identifies one audio file and its metadata. Importing the same canonical path again preserves its identity.
- A **library** contains tracks, playlist folders, and playlists. Folders form a tree; playlists are leaves, and names are unique among siblings. Removing a track from the library removes its playlist references, but never deletes source audio.
- A **playlist folder** contains child folders and playlists. Its identity shares the playlist node ID namespace used by Rekordbox.
- A **playlist** is an ordered list of track identities. Local additions avoid duplicates; existing device entries are retained. Multiple playlists share the same track.
- A **USB library** is a separate snapshot read from a traditional Rekordbox `export.pdb`, including devices prepared by Rekordbox. Its edits never change the local library.
- An **export** publishes selected playlists, unique audio files, waveforms, and `export.pdb`. Sending a playlist creates or updates the USB playlist at the matching folder path while preserving other playlists.

USB playlists can be imported into the local library with copied audio. Library songs can be added to a USB playlist or replace selected entries. USB playlist deletion retains all audio and offers Undo. Existing folders, track records, analysis, and unrelated database tables are preserved. Opening a USB is read-only; writes have automatic backups, exclusive locking, snapshot conflict checks, and staged publication. The user has confirmed USB playlist visibility on their hardware. Analysis, artwork display, sync/quantize and full playback still need physical validation.

A **selection** contains track identities in the currently filtered browser. Shift selects a range in displayed order; Ctrl toggles tracks; Select all selects the full filtered list. A **bulk edit** adds, removes or reorders that selection with one save. The desktop renders only rows around the visible viewport and keeps local/device navigation available together.

A **preview deck** holds one decoded clip, its playback position, pitch and temporary cue point. An overview waveform covers the whole clip; a scrolling waveform follows playback and supports scrubbing. The **preview mixer** combines both decks with channel EQ/level, an equal-power crossfader and master level. Preview reads local or USB audio without changing library metadata, source files or device cues.

**Export progress** reports completed transfer work, including encoder audio time and publication stages. It is monotonic and reaches 100% only after the committed device library is reopened.

A **track analysis** stores aubio beat timestamps and tempo readings and libkeyfinder musical key. New imports analyze automatically; bulk reanalysis keeps identities and playlist membership. BPM/key tags provide fallback when audio has no estimate. Automatic 4/4 bar numbering is a reference, not a verified musical downbeat.

**Artwork** is a cached JPEG extracted by the metadata adapter. Device exports embed it in ID3v2.3 and register 80x80 and 240x240 images without altering existing artwork. USB imports copy the cover locally. The analysis adapter uses established engines; the metadata adapter owns tags and image normalization. See ADR 004.
