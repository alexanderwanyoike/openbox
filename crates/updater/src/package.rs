use crate::UpdateError;

#[derive(Debug, Clone, Copy)]
pub struct Package {
    pub target: &'static str,
    pub asset: &'static str,
    pub binary: &'static str,
}

impl Package {
    pub fn for_target(target: &str) -> Result<Self, UpdateError> {
        let (target, asset, binary) = match target {
            "x86_64-unknown-linux-gnu" => (
                "x86_64-unknown-linux-gnu",
                "openbox-x86_64-unknown-linux-gnu.AppImage",
                "openbox",
            ),
            "x86_64-pc-windows-msvc" => (
                "x86_64-pc-windows-msvc",
                "openbox-x86_64-pc-windows-msvc.zip",
                "openbox.exe",
            ),
            "aarch64-apple-darwin" => (
                "aarch64-apple-darwin",
                "openbox-aarch64-apple-darwin.tar.gz",
                "Openbox.app/Contents/MacOS/openbox",
            ),
            "x86_64-apple-darwin" => (
                "x86_64-apple-darwin",
                "openbox-x86_64-apple-darwin.tar.gz",
                "Openbox.app/Contents/MacOS/openbox",
            ),
            _ => return Err(UpdateError::UnsupportedPlatform(target.to_owned())),
        };
        Ok(Self {
            target,
            asset,
            binary,
        })
    }

    pub fn current() -> Result<Self, UpdateError> {
        Self::for_target(self_update::get_target())
    }
}
