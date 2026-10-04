# XDJ-1000 validation

Status: the user has confirmed playlist visibility on their USB. The new BPM, grid and artwork exports have file-format tests; their behavior on physical players remains pending.

## Target

The original XDJ-1000 and XDJ-1000MK2 use traditional Device Library exports. [Official USB library compatibility](https://rekordbox.com/en/support/usb-export/).

Openbox generates 44.1 kHz stereo CBR MP3 audio, a DeviceSQL `export.pdb`, and DAT/EXT waveform files. New imports receive aubio beat/tempo and libkeyfinder key analysis, and exports include PQTZ beat entries plus registered artwork. Musical downbeat phase is not detected; automatic 4/4 numbering is a reference. Cue export and player sync/quantize remain unvalidated.

## Bench test

Use an empty spare FAT32 USB, selecting the root of its mounted filesystem. Test on the original XDJ-1000 first; record whether the decks are original or MK2 and their firmware versions.

1. Export a small playlist with two real tracks. Include spaces and non-ASCII characters in their titles and artists.
2. Eject the USB through Linux, insert it into the player, and confirm the playlist name, track order, titles, artists, and durations.
3. Load each track, play through its beginning, middle, and end, and test seeking and reverse playback.
4. Check the overview waveform, scrolling waveform, and loading time. Confirm BPM/key and artwork are shown, and beat markers follow audible transients through the full track. Check the initial bar phase separately. Test quantize and sync only after checking the grid.
5. Load the same device through Pro DJ Link from the second player and test playback and seeking there.
6. Reconnect the USB to Openbox. Rename the playlist, change its order, create another playlist, and add existing USB tracks. Eject and verify the edits on the decks.
7. Delete one USB playlist and confirm the remaining playlists and tracks still load. Original audio must remain untouched.
8. Export two playlists sharing a track, and confirm both load that track while only one copy is included in that export.
9. Send one changed playlist again. Confirm it updates while other playlists remain present, and unchanged tracks reuse their audio.
10. Repeat with a copy of an existing Rekordbox-prepared USB. Confirm its folder hierarchy, artwork, cue points, beatgrids, waveforms and original audio still work after playlist rename/reorder, transfers, and deletion/Undo.
11. Replace one playlist entry with a local song. Confirm the original track remains available elsewhere and the replacement loads.
12. Import a USB playlist into the local library, unplug the USB, and confirm the imported audio is available.

## Result record

| Check | Result |
|---|---|
| Model and firmware | Pending |
| USB filesystem and partition layout | Pending |
| Playlist browsing and ordering | Playlist visibility confirmed by user; full ordering check pending |
| Audio loading and playback | Pending |
| Seeking, reverse, and duration | Pending |
| Overview and scrolling waveforms | Pending |
| Detected BPM/key and beat alignment | Pending |
| Artwork display | Pending |
| Bar phase, quantize and sync | Pending |
| Second-player link loading | Pending |
| USB playlist edits and reexport | Pending |
| Existing Rekordbox USB preservation | Pending |
| Library transfers and song replacement | Pending |

Resolve any failure before calling this exporter hardware-ready. An independent parser accepting a file does not prove that the player accepts it.
