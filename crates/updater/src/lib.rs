mod package;
pub use package::Package;

use semver::Version;

pub fn newer_stable(current: &str, candidate: &str) -> Result<bool, semver::Error> {
    let current = Version::parse(current)?;
    let candidate = Version::parse(candidate.trim_start_matches('v'))?;
    Ok(candidate > current && candidate.pre.is_empty() && candidate.build.is_empty())
}

use self_update::backends::github;
use std::{path::Path, time::Duration};

pub const ASSET: &str = "openbox-x86_64-unknown-linux-gnu.tar.gz";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub sha256: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum UpdateError {
    #[error("Updates are unavailable for {0}")]
    UnsupportedPlatform(String),
    #[error("Update service unavailable: {0}")]
    Service(String),
    #[error("The release has no compatible verified Linux download")]
    InvalidRelease,
    #[error("The requested release is not newer than the installed version")]
    NotNewer,
    #[error("Cannot restart Openbox: {0}")]
    Restart(String),
}

#[derive(Debug, Clone)]
pub struct Client {
    api_base: Option<String>,
}
impl Default for Client {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        if let Ok(base) = std::env::var("OPENBOX_UPDATE_TEST_API")
            && base.starts_with("http://127.0.0.1:")
        {
            return Self::with_api_base(&base);
        }
        Self { api_base: None }
    }
}
impl Client {
    pub fn with_api_base(base: &str) -> Self {
        Self {
            api_base: Some(base.to_owned()),
        }
    }

    fn builder(&self, current: &str) -> github::UpdateBuilder {
        let mut builder = github::Update::configure();
        builder
            .repo_owner("alexanderwanyoike")
            .repo_name("openbox")
            .bin_name("openbox")
            .current_version(current)
            .target("x86_64-unknown-linux-gnu")
            .unattended()
            .timeout(Duration::from_secs(120))
            .asset_matcher(|assets| assets.iter().find(|asset| asset.name() == ASSET).cloned());
        if let Some(base) = &self.api_base {
            builder.api_base_url(base);
        }
        builder
    }

    pub fn check(&self, current: &str) -> Result<Option<Release>, UpdateError> {
        let updater = self.builder(current).build().map_err(service)?;
        let releases = match updater.get_latest_release() {
            Ok(releases) => releases,
            Err(error) if error.http_status() == Some(404) => return Ok(None),
            Err(error) => return Err(service(error)),
        };
        let Some(release) = releases.latest() else {
            return Ok(None);
        };
        if !newer_stable(current, release.version()).map_err(service)? {
            return Ok(None);
        }
        if !release.assets().iter().any(|asset| {
            asset.name() == ASSET
                && asset.digest().is_some_and(|digest| {
                    digest.starts_with("sha256:")
                        && digest.len() == 71
                        && digest[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
                })
        }) {
            return Err(UpdateError::InvalidRelease);
        }
        let digest = release
            .assets()
            .iter()
            .find(|asset| asset.name() == ASSET)
            .and_then(|asset| asset.digest())
            .ok_or(UpdateError::InvalidRelease)?;
        Ok(Some(Release {
            version: release.version().to_owned(),
            sha256: digest[7..].to_owned(),
        }))
    }
    pub fn install(
        &self,
        current: &str,
        release: &Release,
        destination: &Path,
        progress: impl Fn(u64, Option<u64>) + Send + Sync + 'static,
    ) -> Result<(), UpdateError> {
        if !newer_stable(current, &release.version).map_err(service)? {
            return Err(UpdateError::NotNewer);
        }
        let expected = format!("openbox {}", release.version);
        self.builder(current)
            .release_tag(format!("v{}", release.version))
            .bin_install_path(destination)
            .verify_checksum(self_update::Checksum::Sha256(release.sha256.clone()))
            .progress_callback(progress)
            .verify_binary(move |path| {
                let output = std::process::Command::new(path)
                    .arg("--version")
                    .output()
                    .map_err(|error| {
                        self_update::Error::verification_rejected(error.to_string())
                    })?;
                if output.status.success()
                    && String::from_utf8_lossy(&output.stdout).trim() == expected
                {
                    Ok(())
                } else {
                    Err(self_update::Error::verification_rejected(
                        "downloaded executable version does not match",
                    ))
                }
            })
            .build()
            .map_err(service)?
            .update()
            .map_err(service)?;
        Ok(())
    }
}
fn service(error: impl std::fmt::Display) -> UpdateError {
    UpdateError::Service(error.to_string())
}

pub fn restart(
    executable: &Path,
    args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>,
) -> Result<(), UpdateError> {
    std::process::Command::new(executable)
        .args(args)
        .spawn()
        .map_err(|error| UpdateError::Restart(error.to_string()))?;
    Ok(())
}
