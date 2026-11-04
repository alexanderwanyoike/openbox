use semver::Version;

pub fn newer_stable(current: &str, candidate: &str) -> Result<bool, semver::Error> {
    let current = Version::parse(current)?;
    let candidate = Version::parse(candidate.trim_start_matches('v'))?;
    Ok(candidate > current && candidate.pre.is_empty() && candidate.build.is_empty())
}

use std::{path::{Path, PathBuf}, time::Duration};
use self_update::backends::github;

pub const ASSET: &str = "openbox-x86_64-unknown-linux-gnu.tar.gz";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release { pub version: String }

#[derive(Debug, Clone, thiserror::Error)]
pub enum UpdateError {
    #[error("Update service unavailable: {0}")] Service(String),
    #[error("The release has no compatible verified Linux download")] InvalidRelease,
    #[error("The requested release is not newer than the installed version")] NotNewer,
    #[error("Cannot restart Openbox: {0}")] Restart(String),
}

#[derive(Debug, Clone)]
pub struct Client { api_base: Option<String> }
impl Default for Client { fn default() -> Self { Self { api_base: None } } }
impl Client {
    #[cfg(debug_assertions)]
    pub fn with_api_base(base: &str) -> Self { Self { api_base: Some(base.to_owned()) } }

    fn builder(&self, current: &str) -> github::UpdateBuilder {
        let mut builder = github::Update::configure();
        builder.repo_owner("alexanderwanyoike").repo_name("openbox").bin_name("openbox")
            .current_version(current).target("x86_64-unknown-linux-gnu").unattended().timeout(Duration::from_secs(120))
            .asset_matcher(|assets| assets.iter().find(|asset| asset.name() == ASSET).cloned());
        if let Some(base) = &self.api_base { builder.api_base_url(base); }
        builder
    }

    pub fn check(&self, current: &str) -> Result<Option<Release>, UpdateError> {
        let updater = self.builder(current).build().map_err(service)?;
        let releases = match updater.get_latest_release() {
            Ok(releases) => releases,
            Err(error) if error.http_status() == Some(404) => return Ok(None),
            Err(error) => return Err(service(error)),
        };
        let Some(release) = releases.latest() else { return Ok(None) };
        if !newer_stable(current, release.version()).map_err(service)? { return Ok(None); }
        if !release.assets().iter().any(|asset| asset.name() == ASSET && asset.digest().is_some_and(|digest| digest.starts_with("sha256:") && digest.len() == 71)) {
            return Err(UpdateError::InvalidRelease);
        }
        Ok(Some(Release { version: release.version().to_owned() }))
    }
}
fn service(error: impl std::fmt::Display) -> UpdateError { UpdateError::Service(error.to_string()) }
