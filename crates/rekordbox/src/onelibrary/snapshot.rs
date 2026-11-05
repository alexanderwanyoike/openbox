use super::super::safe_path;
use super::DATABASE;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub bytes: Vec<u8>,
    pub wal: Option<Vec<u8>>,
}

impl Snapshot {
    pub fn read(root: &Path) -> Result<Option<Self>, String> {
        let Some(bytes) = read_optional(&safe_path(root, DATABASE)?)? else {
            return Ok(None);
        };
        let wal = read_optional(&safe_path(root, &format!("{DATABASE}-wal"))?)?;
        Ok(Some(Self { bytes, wal }))
    }

    pub fn copy_to(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, &self.bytes).map_err(|e| e.to_string())?;
        if let Some(wal) = &self.wal {
            let mut name = path.as_os_str().to_os_string();
            name.push("-wal");
            std::fs::write(name, wal).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

pub(crate) fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
