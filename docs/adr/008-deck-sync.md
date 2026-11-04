# 008: Beat and tempo sync for preview decks

Status: accepted.

Mixxx's [SyncLock developer guide](https://github.com/mixxxdj/mixxx/wiki/Developer-Guide-SyncLock) and [sync control interface](https://github.com/mixxxdj/mixxx/blob/main/src/engine/sync/synccontrol.h) inform the leader/follower model: match effective BPM, align fractional beat position, then apply small speed corrections. No Mixxx engine source is copied. Its source is GPL licensed; the model is implemented using Openbox's existing output callback and analyzed grids.

`openbox-audio` owns validated beat grids, sync errors, follower links, initial nearest-beat phase alignment and bounded two-percent speed bending. Both decks share an output clock. The desktop owns master selection and passes stored analysis into the audio engine; it does not calculate tempo or key. Grid refresh releases sync without relocating either playhead.

Sync requires two loaded tracks with valid analyzed grids and a rate within the existing plus/minus eight-percent pitch range. Missing grids, invalid grids and an out-of-range rate are explicit errors. Initial activation may move the follower to the nearest matching beat phase. While playing, drift correction adjusts speed without cursor jumps. Manual pitch, seeking or cueing releases a follower. Master pitch changes update the follower rate; master seek and cue changes realign it. Loading a new song releases links involving that deck.

This is beat-phase alignment, not bar/downbeat or phrase matching. Analysis currently numbers four beats from its first detected beat; it does not identify musical downbeats. Sync changes pitch with speed and has no key lock. Releasing sync keeps the current nominal tempo. The mixer volume master remains independent from the sync master.

Regression tests cover tempo/phase alignment, ten seconds of shared-clock rendering, master pitch follow, manual release, variable-grid drift correction without playhead jumps and releasing stale links after reanalysis. The native harness loads generated 120/126 BPM tracks, activates B sync to A, records output through an isolated PulseAudio sink and compares the two tone envelopes. Their recorded beat-envelope correlation was 1.000. Actual native screenshots show matched BPM, active Sync and manual-pitch release. Source music and library bytes remain unchanged.
