# 016: Resolve the default library independently of the launch directory

Status: accepted.

The prototype default `.prototype/library.json` depends on the process working directory. A packaged app launched from Finder, Explorer or a read-only installation directory cannot rely on that location being writable or consistent between launches.

Use [directories 6 ProjectDirs](https://docs.rs/directories/6.0.0/directories/struct.ProjectDirs.html) to resolve the platform's local application data directory for `io.github.alexanderwanyoike.openbox`. The desktop startup adapter owns this choice. A missing data directory is an explicit startup error explaining `--library FILE` as the alternative. The domain and persistence crates retain their existing path contracts.

An explicit `--library` continues to take precedence, including relative paths supplied deliberately. `--demo` keeps the separate generated demo file. Existing prototype files are not migrated or modified automatically; they can be opened with the explicit flag. The user's existing launcher already supplies its library file and is unaffected.

The failing absolute-path regression was committed before the implementation. Native Linux verification started without `--library` from a read-only directory, created a playlist through the actual UI and found it under a private `XDG_DATA_HOME`. Restarting from a second read-only directory reopened that playlist. Neither launch directory gained files. This used a private display and disposable data directory, without changing the user's library or desktop session.
