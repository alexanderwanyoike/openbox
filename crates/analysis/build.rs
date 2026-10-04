use std::{fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=src/keyfinder.cpp");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++11")
        .include("vendor/libkeyfinder/src")
        .include("vendor/fftw")
        .file("src/keyfinder.cpp");
    for entry in fs::read_dir("vendor/libkeyfinder/src").unwrap() {
        let path = entry.unwrap().path();
        if path.extension() == Some(std::ffi::OsStr::new("cpp")) {
            build.file(path);
        }
    }
    build.compile("openbox_keyfinder");
    if pkg_config::Config::new().probe("fftw3").is_err() {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
            // Debian separates the runtime SONAME from the development linker symlink.
            println!("cargo:rustc-link-lib=dylib:+verbatim=libfftw3.so.3");
        } else {
            panic!("Install FFTW3 development libraries (pkg-config fftw3)");
        }
    }
    assert!(Path::new("vendor/libkeyfinder/LICENSE").exists());
}
