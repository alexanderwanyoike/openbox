# Analysis dependencies

`libkeyfinder/src` and `libkeyfinder/LICENSE` are unmodified files from Mixxx's libkeyfinder 2.2.8, commit `b33b5a88e04a5182dd19c38c57762925631118fd`:

https://github.com/mixxxdj/libkeyfinder/tree/b33b5a88e04a5182dd19c38c57762925631118fd

`fftw/fftw3.h` and `fftw/COPYING` are from FFTW 3.3.8 as distributed in `fftw-src` 0.8.0. Only the header is vendored; the application links the platform's FFTW3 runtime. The header declares the upstream API and does not contain an Openbox FFT implementation.

https://www.fftw.org/

The C++ bridge is Openbox code under the workspace license. It calls libkeyfinder's progressive analysis API and catches exceptions at the language boundary. Detection code belongs to the upstream libraries.
