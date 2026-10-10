//! CLI config file.

use {
    mollusk_svm::result::Compare,
    serde::{Deserialize, Serialize},
};

/// Config file for configuring CLI commands.
///
/// For now, only used to configure fixture testing (ie. `execute-fixture` and
/// `run-test`)
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFile {
    pub checks: Vec<Compare>,
}

impl ConfigFile {
    /// Load the config file from a JSON file at the given path.
    fn load_json(path: &str) -> Result<Self, String> {
        let file = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&file).map_err(|e| e.to_string())
    }

    /// Load the config file from a YAML file at the given path.
    fn load_yaml(path: &str) -> Result<Self, String> {
        let file = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_yaml::from_str(&file).map_err(|e| e.to_string())
    }

    pub fn try_load(path: &str) -> Result<ConfigFile, Box<dyn std::error::Error>> {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| format!("Config file must have a .json or .yaml extension: {}", path))?;
        match ext {
            "json" => Self::load_json(path).map_err(|e| e.into()),
            "yaml" => Self::load_yaml(path).map_err(|e| e.into()),
            _ => Err(format!("Unsupported config file format: {}", ext).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ConfigFile;

    #[test]
    fn try_load_rejects_path_without_extension() {
        let Err(err) = ConfigFile::try_load("settings") else {
            panic!("expected an error");
        };
        assert_eq!(
            err.to_string(),
            "Config file must have a .json or .yaml extension: settings"
        );
    }

    #[test]
    fn try_load_rejects_unsupported_extension() {
        let Err(err) = ConfigFile::try_load("settings.toml") else {
            panic!("expected an error");
        };
        assert_eq!(err.to_string(), "Unsupported config file format: toml");
    }
}
