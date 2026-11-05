# Third-party analysis components

Openbox-authored source files retain their MIT license. The application links GPL components, so GPLv3 requirements apply when distributing the combined executable. Include the applicable licenses and corresponding source for the complete combined program when distributing binaries. This repository does not distribute a compiled binary.

| Component | Version / source | License |
|---|---|---|
| aubio, via bliss-audio-aubio-rs and bliss-audio-aubio-sys | 0.2.4 Rust bindings, source available through the locked Cargo dependencies | GPLv3 |
| libkeyfinder | 2.2.8, unmodified vendored source at `crates/analysis/vendor/libkeyfinder` | GPLv3 or later |
| FFTW3 | Platform runtime, vendored 3.3.8 API header at `crates/analysis/vendor/fftw` | GPLv2 or later |
| Lofty | 0.25.4, Cargo dependency | MIT or Apache-2.0 |
| image | 0.25, Cargo dependency | MIT or Apache-2.0 |

Upstream references: [aubio](https://aubio.org/), [libkeyfinder](https://github.com/mixxxdj/libkeyfinder), [FFTW](https://www.fftw.org/), [Lofty](https://github.com/Serial-ATA/lofty-rs), [image](https://github.com/image-rs/image).

The libkeyfinder and FFTW license texts are included alongside the vendored files. Cargo.lock identifies the exact Rust dependencies. See their published crate packages for their original notices and source.

## OneLibrary

The OneLibrary schema and device key decoding are adapted from [Baken](https://github.com/M-Igashi/baken/tree/8446116b51dba57370c37ff3bcc1efd69ab9ab40), commit `8446116b51dba57370c37ff3bcc1efd69ab9ab40`, under MIT. The key encoding originates in [pyrekordbox](https://github.com/dylanljones/pyrekordbox), also MIT. Their license texts are included in `crates/rekordbox/src/onelibrary/`. Device database encryption and authentication use SQLCipher through rusqlite, with SQLCipher 4 defaults. The exact SQLCipher and OpenSSL source versions are locked in Cargo.lock and must accompany corresponding-source distributions.
