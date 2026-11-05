# 014: Publish the generated modern waveform file

Status: accepted. Extends ADR 004 and ADR 010.

The pinned Baken generator already returns DAT, EXT and 2EX files, but Openbox discarded the third result. Its `anlz/generate/assemble.rs` and `waveform.rs`, revision `2789f0615236affdf65d856171b58dfd1eb2425d`, are the established implementation for this change. No new waveform encoder is introduced.

Stage and publish all three siblings. DAT retains the beatgrid, including MP3 priming compensation; EXT retains the existing color waveform; 2EX supplies PWV7, PWV6 and PWVC sections. All three identify the same device audio path. Existing device assets are preserved.

Prepared-source links now record the three analysis paths. Reuse requires all three files to exist beneath the validated device root. Older cache entries default to no recorded paths and are prepared again. This creates new staged assets while retaining the previous audio and analysis. Subsequent exports reuse the complete set.

Both the missing-file and older-cache regressions failed before their respective implementation commits. Independent rekordcrate parsing checks the modern file and section tags. The manual production-API CLI verified a generated 120 BPM track, 64 evenly spaced PQTZ beats, all three modern sections, both database formats and an unchanged source hash. This is file-level verification; player rendering remains subject to the hardware checklist.
