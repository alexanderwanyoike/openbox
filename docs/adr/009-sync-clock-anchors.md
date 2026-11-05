# 009: Preserve the clock paired with a deck position

Status: accepted. Depends on ADR 008 and the waveform reference-time model in ADR 005.

## Failure

A tempo-only sync operation copied the master's timestamp into the follower even when its playhead did not move. A stopped master can have an old timestamp. Starting a linked follower then made the waveform count the master's idle time as elapsed playback. Its position was clamped to the clip end although the audio cursor still showed the beginning.

The failure was reproduced through the actual Mixer API and in the installed native application. The native fixture kept A stopped, armed B's sync, then started B while a private output sink was suspended. Delaying that private callback exposed the stale reference deterministically: the overview playhead appeared at pixel 557 of 558 while the deck's audio position remained 00:00. The fixture uses generated audio and leaves the user's desktop and output devices untouched.

This reproduction establishes a visual clock defect. It does not establish that the user's audio itself accelerated.

## Decision

A position and its reference instant are one timing observation. Follow the same paired-value/reference-time pattern used by [Mixxx VisualPlayPosition](https://github.com/mixxxdj/mixxx/blob/main/src/waveform/visualplayposition.cpp). No Mixxx source is copied; its GPL licensing is unchanged.

The audio API owns each DeckSnapshot timestamp. The CPAL callback anchors the cursor to the end of queued output, including playback lead and buffer duration. Transport operations set a new anchor when they change the position or playing state.

Tempo-only sync retains the follower's existing anchor. Phase alignment between playing decks moves its position and transfers the playing master's anchor with that position. This preserves the coordinate/time relationship without limiting or masking normal waveform interpolation.

The producer code already used by the combined preview is included at the audio boundary so this contract can be regression-tested on the sync branch independently of the desktop consumer.

## Verification

The failing clock-preservation regression was committed before the fix. Coverage also checks starting a linked follower after its master stops and transferring the master's clock when phase alignment actually repositions the follower.

The native Xvfb regression runs against the combined preview containing PR #3. It suspends only its own generated PulseAudio sink, asserts that the waveform begins near the start after Play, and compares library and source bytes afterward. Snapshot/audio tests distinguish cursor movement from visual interpolation.

Merge PR #5 before this dependent fix. When combining PR #3, retain the conditional anchor transfer and the audio-owned snapshot timestamp.
