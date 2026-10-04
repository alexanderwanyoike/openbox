# 003: Isolate preview audio and report transfer progress

Status: accepted. Extends ADR 002 and replaces the earlier exclusion of preview playback.

Two preview decks help audition music and test transitions while preparing playlists. They must remain usable during background export, and preview must never change music files or device performance data.

`openbox-audio` owns immutable decoded clips, waveform envelopes, deck transport, channel EQ, level controls, mixing and default audio output. It has no dependency on the library, Rekordbox or Iced crates. Its public mixer renders samples deterministically, using the same path as the CPAL output callback. Tests exercise actual mixed samples instead of mocking an audio sink.

FFmpeg decodes only chosen songs into stereo 44.1 kHz signed 16-bit PCM. Clips share their buffers and envelope data through `Arc`; decoded files are bounded to 30 minutes. This allows immediate seeking without introducing a streaming cache or another decoder dependency. Decode and audio initialization run outside Iced's event thread. Each deck load has a generation number, so a stale decode cannot replace a newer selection.

One CPAL stream renders both decks on a shared clock. Its owner thread retains the platform stream until the final engine handle drops. The callback uses a fixed scratch buffer and a nonblocking mixer lock, avoiding allocation and waiting for GUI work. The UI reads snapshots; playback position advances from rendered audio rather than a UI timer. Hiding the panel does not interrupt playback.

Preview offers speed/pitch adjustment, session cue points, a simple three-band crossover EQ and an equal-power crossfader. It does not imply beat detection, tempo lock, automatic sync, separate headphone routing or compatibility with performance cues stored by Rekordbox.

The device adapter adds optional progress callbacks alongside its existing transfer entry points. Progress uses encoder audio time within each track, completed waveform work, publication and database commit stages. Reused tracks advance after their fingerprints are checked. The desktop samples the latest progress value without queuing every encoder update. Only a successful reopen publishes 100%; failures clear the bar and retain the existing error flow. Progress reporting does not change locking, backups, staging or rollback rules.
