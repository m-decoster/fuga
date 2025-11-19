use std::collections::HashMap;

/// Representation of an Application to be launched.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct LaunchFile {
    /// Name of the application.
    pub name: String,
    /// Environment variables.
    pub env: Option<HashMap<String, String>>,
    /// List of processes to be launched.
    pub processes: Vec<ProcessDescription>,
}

impl LaunchFile {
    /// Parse a launch file from a string.
    ///
    /// Args:
    ///    toml_str: The TOML string representing the launch file.
    ///
    /// Returns:
    ///     Ok(Application) if parsing is successful, Err(toml::de::Error) otherwise.
    pub fn from_str(toml_str: &str) -> std::io::Result<Self> {
        toml::from_str(toml_str)
            .map_err(|e| std::io::Error::other(format!("Error parsing TOML file: {}", e.message())))
    }

    /// Parses a launch file from a file path.
    ///
    /// Args:
    ///    file_path: The path to the launch file.
    ///
    /// Returns:
    ///     Ok(Application) if parsing is successful.
    pub fn from_file(file_path: &str) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(file_path)?;
        Self::from_str(&content)
    }
}

/// Representation of a process to be launched.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct ProcessDescription {
    /// A unique name for the process.
    pub name: String,
    /// Command to start the process.
    pub command: String,
    /// Working directory, in which the process is to be launched.
    /// Defaults to the current working directory.
    pub work_dir: Option<String>,
    /// Arguments to pass to the process.
    pub args: Vec<String>,
    /// Whether the process should be restarted. If None, defaults to Restart::Never.
    pub restart: Option<Restart>,
    /// How long to wait before restarting the process. If None, defaults to 0 (immediate restart).
    pub restart_delay_secs: Option<f32>,
    /// File to which stdout will be written.
    /// When this value is empty, no file will be written.
    pub log_file_out: Option<String>,
    /// File to which stderr will be written.
    /// When this value is empty, no file will be written.
    pub log_file_err: Option<String>,
}

/// When to restart a process.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Restart {
    /// When the process exited with a non-zero status.
    #[serde(rename = "on-failure")]
    OnFailure,
    /// Always restart the process.
    #[serde(rename = "always")]
    Always,
    /// Never restart the process.
    #[serde(rename = "never")]
    Never,
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml;
    #[test]
    fn test_parse_launch_file() {
        let toml_str = r#"
            name = "MyApp"

            [[processes]]
            name = "App1"
            command = "python"
            args = ["app1.py"]
            restart = "always"
            restart_delay_secs = 5.0

            [[processes]]
            name = "App2"
            command = "node"
            args = ["app2.js"]
            restart = "never"
        "#;
        let launch_file: LaunchFile = toml::from_str(toml_str).unwrap();
        assert_eq!(launch_file.name, "MyApp");
        assert_eq!(launch_file.processes.len(), 2);
        assert_eq!(launch_file.processes[0].name, "App1");
        assert_eq!(launch_file.processes[0].restart, Some(Restart::Always));
        assert_eq!(launch_file.processes[0].restart_delay_secs, Some(5.0));
    }
}
