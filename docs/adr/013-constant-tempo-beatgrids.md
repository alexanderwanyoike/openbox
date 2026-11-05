# 013: Stabilize detected beatgrids before device export

Status: accepted. Extends ADR 004 and ADR 008.

A generated 120 BPM recording was reported as 120.461 BPM, with individual detector timestamps used directly as the grid. That jitter causes varying grid intervals and can make deck sync or CDJ quantization inconsistent even when the recording has a constant tempo.

Adapt Mixxx [BeatUtils](https://github.com/mixxxdj/mixxx/blob/bc49961fd3e5a46cd46588090c652a0d7ec44e18/src/track/beatutils.cpp), pinned at `bc49961fd3e5a46cd46588090c652a0d7ec44e18`. Aubio still detects beats, and libkeyfinder still detects key. The Rust post-processing retains Mixxx's 25 ms phase tolerance, 100 ms accumulated error limit, single-outlier allowance, constant-region selection, compatible-region combination, tempo rounding and phase adjustment. GPLv2-or-later attribution and terms are included beside the adapted source.

A region must contain at least 16 intervals to anchor a constant grid. Sparse or unstable detections retain the existing detector output. A stable region produces evenly spaced beats through the decoded duration, including intro, outro and missing-beat sections. Musical bar numbers remain a 4/4 reference; this does not detect downbeats or time signatures. Dynamic-tempo recordings are not guaranteed to fit this normal, constant-tempo analysis mode.

The analysis version changes so new imports and explicit reanalysis record the improved grid. Export refreshes missing or outdated Openbox-produced analysis without changing the local library. Its source fingerprint includes the engine version when analysis must be regenerated, so an older cached export cannot bypass this update. Imported Rekordbox and other externally supplied grids are preserved. Repeated exports with the same engine reuse the already prepared audio and analysis.

The USB adapter still owns PQTZ serialization and MP3 priming compensation. A separate parser, rekordcrate, verifies 12000 tempo units and 500 ms intervals in an exported 120 BPM grid. Long-grid tests cover a missing-beat region, an outlier and fractional BPM without forcing an integer tempo.

Manual verification used a standalone CLI calling the production analysis and export APIs with generated audio. It detected 120 BPM and C major, wrote both device databases, reopened the USB playlist and independently parsed 64 evenly spaced PQTZ beats. The original audio hash was unchanged. These checks establish file-level behavior; CDJ waveform alignment and quantization still require physical hardware validation.
