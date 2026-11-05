use crate::UpdateError;
use std::path::PathBuf;

pub fn installation_path() -> Result<PathBuf, UpdateError> {
    let executable = std::env::current_exe().map_err(crate::service)?;
    #[cfg(target_os = "linux")]
    {
        use std::io::Read;
        let appdir = std::env::var_os("APPDIR").and_then(|path| std::fs::canonicalize(path).ok());
        if appdir.is_some_and(|path| executable.starts_with(path)) {
            let outer = std::env::var_os("APPIMAGE")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .and_then(|path| path.canonicalize().ok())
                .ok_or(UpdateError::InvalidAppImage)?;
            let mut header = [0; 11];
            std::fs::File::open(&outer)
                .and_then(|mut file| file.read_exact(&mut header))
                .map_err(|_| UpdateError::InvalidAppImage)?;
            if &header[..4] != b"\x7fELF" || &header[8..] != b"AI\x02" {
                return Err(UpdateError::InvalidAppImage);
            }
            return Ok(outer);
        }
    }
    Ok(executable)
}
