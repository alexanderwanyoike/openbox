# Third-party analysis components

Openbox-authored source files retain their MIT license. The application links GPL components, so GPLv3 requirements apply when distributing the combined executable. Include the applicable licenses and corresponding source for the complete combined program when distributing binaries. Release source bundles include the repository, vendored Cargo dependencies and the complete FFTW source for the distributed executable. Binary archives include dependency notices and a package inventory in `licenses/`, or `Contents/Resources/licenses/` in the Mac app.

| Component | Version / source | License |
|---|---|---|
| aubio, via bliss-audio-aubio-rs and bliss-audio-aubio-sys | 0.2.4 Rust bindings, source available through the locked Cargo dependencies | GPLv3 |
| libkeyfinder | 2.2.8, unmodified vendored source at `crates/analysis/vendor/libkeyfinder` | GPLv3 or later |
| FFTW3 | Release builds: pinned 3.3.10 source; vendored 3.3.8 API header at `crates/analysis/vendor/fftw` | GPLv2 or later |
| Mixxx BeatUtils | Rust adaptation of `src/track/beatutils.cpp`, commit `bc49961fd3e5a46cd46588090c652a0d7ec44e18` | GPLv2 or later |
| Lofty | 0.25.4, Cargo dependency | MIT or Apache-2.0 |
| image | 0.25, Cargo dependency | MIT or Apache-2.0 |

Upstream references: [aubio](https://aubio.org/), [libkeyfinder](https://github.com/mixxxdj/libkeyfinder), [FFTW](https://www.fftw.org/), [Lofty](https://github.com/Serial-ATA/lofty-rs), [image](https://github.com/image-rs/image).

The libkeyfinder and FFTW license texts are included alongside the vendored files. Cargo.lock identifies the exact Rust dependencies. See their published crate packages for their original notices and source.

The grid post-processing in `crates/analysis/src/grid` adapts Mixxx's constant-region selection, tempo rounding and phase adjustment. Copyright 2001-2026 Mixxx Development Team. The adaptation is identified as modified and retains GPLv2-or-later terms in `LICENSE.mixxx` beside the source. Aubio remains the beat detector; libkeyfinder remains the key detector.

The update adapter uses `self_update` 1.3.3 and `self-replace` 1.5 under MIT, with `semver` under MIT or Apache-2.0. Their sources and notices are included in the locked Cargo dependencies and corresponding-source bundle.
## Native interface

The application menus use [Iced AW](https://github.com/iced-rs/iced_aw), MIT licensed, with only its menu feature enabled. The bundled Manrope Regular, SemiBold and Bold faces are static instances of the project's Manrope variable font. Copyright 2019 The Manrope Project Authors. The SIL Open Font License is included in `resources/fonts/Manrope-LICENSE.txt`.


## OneLibrary

The OneLibrary schema and device key decoding are adapted from [Baken](https://github.com/M-Igashi/baken/tree/8446116b51dba57370c37ff3bcc1efd69ab9ab40), commit `8446116b51dba57370c37ff3bcc1efd69ab9ab40`, under MIT. The key encoding originates in [pyrekordbox](https://github.com/dylanljones/pyrekordbox), also MIT. Their license texts are included in `crates/rekordbox/src/onelibrary/`. Device database encryption and authentication use SQLCipher through rusqlite, with SQLCipher 4 defaults. The exact SQLCipher and OpenSSL source versions are locked in Cargo.lock and must accompany corresponding-source distributions.
