fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=resources/icons/openbox.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("resources/icons/openbox.ico")
            .set("ProductName", "Openbox")
            .set("FileDescription", "Openbox")
            .compile()?;
    }
    Ok(())
}
