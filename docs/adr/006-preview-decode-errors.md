# 006: Typed preview decoder failures

Status: accepted.

Preview loading previously returned raw FFmpeg stderr as its application error. A missing file exposed decoder addresses and internal diagnostics without an actionable explanation.

The audio boundary now returns a typed DecodeError. Missing and unreadable files, an unavailable decoder, unsupported clip length and invalid PCM are distinct expected failures. Its Display implementation describes the action the user can take and includes the filename when relevant. Decoder diagnostics remain available in the structured error for troubleshooting. The UI boundary converts the error to a display message.

This follows Rust's standard typed Error and Result pattern and keeps decoder concerns in the audio crate. No new dependency or custom decoding algorithm is introduced. The existing FFmpeg process remains responsible for decoding.

The regression first failed on the raw decoder message. Missing-file guidance, corrupt input and successful decoding are covered by audio tests. Native verification uses a disposable missing-file fixture with the notice UI from PR #3; it does not change library records or source files.
