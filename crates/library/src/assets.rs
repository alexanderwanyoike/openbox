use std::{fs::File, path::Path};

pub(super) fn copy(mut source: File, destination: &Path) -> Result<(), String> {
    // fs::copy would inherit read-only USB permissions and prevent flushing on Windows.
    let mut output = File::create(destination).map_err(|error| error.to_string())?;
    std::io::copy(&mut source, &mut output).map_err(|error| error.to_string())?;
    output.sync_all().map_err(|error| error.to_string())
}
