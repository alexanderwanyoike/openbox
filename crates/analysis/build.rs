use std::{fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=src/keyfinder.cpp");
    println!("cargo:rerun-if-env-changed=OPENBOX_FFTW_SOURCE");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++14")
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
    if let Some(source) = std::env::var_os("OPENBOX_FFTW_SOURCE") {
        let installed = cmake::Config::new(source)
            .define("BUILD_SHARED_LIBS", "OFF")
            .define("BUILD_TESTS", "OFF")
            .define("DISABLE_FORTRAN", "ON")
            .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
            .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
            .define("CMAKE_INSTALL_LIBDIR", "lib")
            .build();
        println!(
            "cargo:rustc-link-search=native={}",
            installed.join("lib").display()
        );
        println!("cargo:rustc-link-lib=static=fftw3");
    } else if pkg_config::Config::new().probe("fftw3").is_err() {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
            // Debian separates the runtime SONAME from the development linker symlink.
            println!("cargo:rustc-link-lib=dylib:+verbatim=libfftw3.so.3");
        } else {
            panic!("Set OPENBOX_FFTW_SOURCE to FFTW 3.3.10 source, or install pkg-config fftw3");
        }
    }
    assert!(Path::new("vendor/libkeyfinder/LICENSE").exists());
}
