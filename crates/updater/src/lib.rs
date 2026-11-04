use semver::Version;

pub fn newer_stable(current: &str, candidate: &str) -> Result<bool, semver::Error> {
    let current = Version::parse(current)?;
    let candidate = Version::parse(candidate.trim_start_matches('v'))?;
    Ok(candidate > current && candidate.pre.is_empty() && candidate.build.is_empty())
}
