# 008: Beat and tempo sync for preview decks

Status: accepted.

Mixxx's [BpmControl](https://github.com/mixxxdj/mixxx/blob/main/src/engine/controls/bpmcontrol.cpp), [SyncControl](https://github.com/mixxxdj/mixxx/blob/main/src/engine/sync/synccontrol.cpp), [EngineSync](https://github.com/mixxxdj/mixxx/blob/main/src/engine/sync/enginesync.cpp) and [Beats](https://github.com/mixxxdj/mixxx/blob/main/src/track/beats.cpp) implementations inform this change. The source was checked directly because its SyncLock wiki is marked as awaiting review. No Mixxx engine source is copied. Mixxx is GPL licensed; Openbox's existing combined executable is already subject to the GPL components listed in THIRD-PARTY-NOTICES.md.

## Responsibilities and clock

The audio crate owns validated beat maps, follower links, tempo ratios, initial phase alignment and bounded rate correction. Both decks share the output callback clock. The desktop selects the master and passes stored analysis into the engine. No new BPM or key detector is implemented. Library records, audio files and USB data are unchanged by sync.

Tempo matching uses the current eight-interval beat neighborhood, following Mixxx's local-tempo model. The ratio refreshes during rendering. Phase correction then handles the remaining fractional-beat offset. Its proportional gain of 0.7, one-percent beat deadband, five-percent bend cap and two-percentage-point per-callback slew limit follow Mixxx's bounded controller pattern, using the shortest phase-error direction. Continuous correction changes speed without seeking. After the last detected beat, the clock extrapolates using the last local tempo. Audio snapshots provide the effective local BPM to the readout.

The original whole-track BPM ratio left phase correction compensating for differences in local tempo. Its two-percent bend cap also acquired large offsets too slowly. A regression using anonymized detector timings reproduced a 0.1854-beat error after ten seconds. The corrected controller acquires that grid by the same deadline without cursor jumps.

## Transport and fallback

The first playing deck becomes master. When the master stops, the other playing deck takes over. Explicit master handover preserves the active pair. Sync on a stopped follower preserves its cue and aligns on Play, matching Mixxx's stopped-deck behavior. Cue and Set cue retain the link. Manual pitch or seeking releases a follower. Loading another song or replacing analysis releases affected links without relocating the other deck.

The supported nominal pitch range remains plus/minus eight percent. Missing/invalid grids and incompatible track tempos return typed errors. Brief early double-tempo estimates can make a local ratio unusable despite compatible track averages. Initial arming can use that compatible average ratio. During an unusable local span, rendering retains the last valid tempo/link and suspends phase bending. A later usable span resumes beat following. A manual master pitch change outside the supported range releases its follower. Expected range failures use Result explicitly.

The button shows TEMPO while awaiting Play or usable local data and SYNC ON during active phase following. Sync depends on an accurate musical beat grid. It provides beat-phase alignment; musical downbeats and phrases are not detected. Speed changes also change pitch because key lock is not implemented. Releasing sync retains the nominal tempo. Mixer master volume remains independent of the sync master.

## Verification

Regressions first reproduced local-tempo drift, stopped-cue movement, B-first failure, mismatched readouts, grid-ending drift, intro rejection, slow phase acquisition and cue disarming. The timing fixture contains no audio, paths, titles or artists.

Native Xvfb verification records generated pulses through a private PulseAudio sink. It checks B-first playback, matching readouts, master handover with a pitch change, pause/restart, Cue/Set cue and manual-pitch release. The fixture deliberately reports identical average BPM for tracks with different actual beat intervals. Readout glyph checks tolerate a one-pixel antialiasing edge and verify that the readouts differ before sync. Library and audio bytes are compared afterward.

A separate read-only probe renders two real library tracks from 0, 30 and 60 seconds. Across each 40-second run the link remains active. After the intro settles, peak grid-phase errors are 0.0131, 0.0179 and 0.0134 beats respectively. This measures following the stored grid; musical grid accuracy still needs listening. The user's running desktop session is untouched.
