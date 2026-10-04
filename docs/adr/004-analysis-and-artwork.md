# 004: Established analysis engines and device metadata

## Decision

`openbox-analysis` owns streaming audio analysis, aubio beat tracking, and a small exception-safe bridge to Mixxx's libkeyfinder. `openbox-metadata` owns Lofty tag access, cover normalization, content-addressed artwork files, and ID3v2.3 writing. Domain models store portable metadata and detected beats without either adapter's dependencies. Library imports coordinate both adapters in a background task; the USB adapter consumes their results.

Analysis runs on new imports and explicit bulk reanalysis. Existing track identities and playlist membership survive reanalysis. An export of a legacy track without analysis runs the engines before encoding. Silence and short material can have no detected grid; the UI reports that count. Source files remain read-only.

Beat timestamps come from aubio, and global BPM is the median of its reported BPM values. Key names come from libkeyfinder's key enum. Openbox does not implement a detection algorithm. Beats are numbered cyclically in 4/4 from the first detected beat; this is an automatic bar reference, not an inferred musical downbeat. No sync or quantize claim follows from detection alone.

The device adapter writes exact detected beat entries into PQTZ and precise BPM/key into DeviceSQL. New CBR MP3 files omit Xing, so grid timestamps include LAME's 576 encoder and 529 decoder priming samples at 44.1 kHz. An independent impulse test checks the resulting grid against decoded exported audio to within one millisecond.

Covers are cached as JPEG files of at most 500 pixels per dimension. Export embeds the cover in ID3v2.3, stages 80x80 and 240x240 device artwork, and appends artwork rows with IDs above existing device IDs. Imported USB covers are copied locally alongside the audio. Cached artwork and new device assets never overwrite original audio or existing device artwork.

## Consequences

Linux builds need a C++11 compiler and the FFTW3 runtime. Aubio and libkeyfinder make GPLv3 distribution requirements applicable to the combined executable; Openbox-authored sources retain MIT. See THIRD-PARTY-NOTICES.md. The native libraries were chosen for established music-analysis use rather than implementing new algorithms or loading large model runtimes.

Detected tempo may have octave ambiguity, and musical key and bar phase are estimates. Device grids and artwork are independently parsed in automated tests, but their playback display, quantize, and sync still need checks on the user's XDJ-1000 hardware.
