use anyhow::{anyhow, Error};

/// Extracts the version from a release filename
pub(crate) fn extract_version_from_release(release: &str) -> Result<String, Error> {
    let re = regex::Regex::new(r"v\d+\.\d+\.\d+").unwrap();
    let captures = re
        .captures(release)
        .ok_or_else(|| anyhow!("Could not extract version from release"))?;

    Ok(captures.get(0).unwrap().as_str().to_string())
}
