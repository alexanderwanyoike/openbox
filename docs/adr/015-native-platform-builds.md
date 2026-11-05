# 015: Build and test each supported native platform

Status: accepted.

The Linux-only build contained a Linux window setting on every platform, lacked Apple Silicon aubio bindings and assumed a system FFTW installation. Native builds exposed these problems directly. The same matrix then exposed Unix-only updater fixtures, platform-specific command modifiers, aliased temporary paths and a Windows file-flush failure.

Use native GitHub runners for Linux x86_64, Windows MSVC x86_64, macOS Apple Silicon and macOS Intel. Runner labels follow the [official runner inventory](https://github.com/actions/runner-images). Each job builds the executable, runs the workspace tests and exercises `--version` and `--help`. Linux also retains the default GPU-renderer lint and test checks. Feature PRs run these checks once per workflow; dependency compilation is cached and Linux jobs bound compiler concurrency and omit debug symbols to avoid excessive resource use.

Release analysis links a static FFTW 3.3.10 built through the upstream CMake configuration, selected by `OPENBOX_FFTW_SOURCE`. CI verifies the official source archive's SHA-256 before extraction. The hash is `56c932549852cddcfafdab3820b0200c7742675be92179e59e6215b340e26467`. The ordinary system pkg-config path remains available for local Linux development. The C++ bridge uses C++14 for MSVC compatibility. Apple Silicon enables aubio's existing bindgen feature because the upstream crate does not ship bindings for that target. Neither detector algorithm is replaced.

Only Linux configures Iced's `application_id` field; other platforms retain their native defaults. Updater verification uses a small compiled native fixture instead of a shell script. Tests use the platform's command modifier and canonical filesystem identities, matching the adapters' existing path contract.

Windows [FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers) requires a writable handle. File publication and copied artwork therefore flush through writable handles to their own prepared files. This neither truncates those files nor opens original music for writing.

USB imports copy content through a newly created writable file, retaining that handle for flushing. `fs::copy` would also inherit read-only source permissions, preventing the writable reopen. The read-only artwork regression verifies successful import and unchanged source permissions.

The workflow was committed before the platform fixes, and its failing target builds and tests were observed on their actual operating systems. Local verification also runs the full software-renderer workspace suite and strict Clippy with static FFTW. Native Linux UI verification uses generated fixtures and a private display and output sink. Interactive macOS and Windows desktop verification is not available on this Linux host; native automated checks do not establish their desktop appearance or physical CDJ compatibility.
