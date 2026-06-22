use anyhow::{Result, bail};
use serde::Deserialize;

const DEFAULT_FORMAT: &str = "%Y-%m-%d_%H-%M";
const DEFAULT_REGEX: &str = r"[a-z_]*_([0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]{2}-[0-9]{2})\..*";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SidecarConfig {
    pub suffix: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaggerConfig {
    pub regex: Option<String>,
    pub format: Option<String>,
    #[serde(default)]
    pub sidecar: Vec<SidecarConfig>,
}

impl StaggerConfig {
    /// Return the configured date extraction regex or the default pattern.
    pub fn regex(&self) -> String {
        self.regex.clone().unwrap_or(DEFAULT_REGEX.to_string())
    }

    /// Return the configured date format or the default format string.
    pub fn format(&self) -> String {
        self.format.clone().unwrap_or(DEFAULT_FORMAT.to_string())
    }

    /// Validate semantic constraints that serde alone cannot express.
    pub fn validate(&self) -> Result<()> {
        for sidecar in &self.sidecar {
            if sidecar.suffix.is_empty() {
                bail!("Configured sidecar suffixes must not be empty");
            }
        }

        Ok(())
    }
}
