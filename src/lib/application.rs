//! Representation of applications as they are parsed by the client and processed by the daemon.

/// Representation of an Application to be launched.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Application {
    /// Name of the application.
    name: String,
    /// List of processes to be launched.
    processes: Vec<Process>,
}

impl Application {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn processes(&self) -> &[Process] {
        &self.processes
    }

    /// Parse a launch file from a string.
    ///
    /// Args:
    ///    toml_str: The TOML string representing the launch file.
    ///
    /// Returns:
    ///     Ok(Application) if parsing is successful, Err(toml::de::Error) otherwise.
    pub fn from_str(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Parse an application file from a file path.
    ///
    /// Args:
    ///    file_path: The path to the application file.
    ///
    /// Returns:
    ///     Ok(Application) if parsing is successful, Err(String) with an error otherwise.
    pub fn from_file(file_path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read application file: {}", e))?;
        Self::from_str(&content)
            .map_err(|e| format!("Failed to parse application file: {}", e))
    }
}

/// Representation of a process to be launched.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Process {
    /// A unique name for the process.
    name: String,
    /// Command to start the process.
    command: String,
    /// Arguments to pass to the process.
    args: Vec<String>,
    /// Whether the process should be restarted. If None, defaults to Restart::Never.
    restart: Option<Restart>,
    /// How long to wait before restarting the process. If None, defaults to 0 (immediate restart).
    restart_delay_secs: Option<f32>,
}

impl Process {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn restart(&self) -> &Option<Restart> {
        &self.restart
    }

    pub fn restart_delay_secs(&self) -> &Option<f32> {
        &self.restart_delay_secs
    }
}

/// When to restart a process.
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
pub enum Restart {
    /// When the process exited with a non-zero status.
    #[serde(rename = "on-failure")]
    OnFailure,
    /// Always restart the process.
    #[serde(rename = "always")]
    Always,
    /// Never restart the process.
    #[serde(rename = "never")]
    Never
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
        let launch_file: Application = toml::from_str(toml_str).unwrap();
        assert_eq!(launch_file.name, "MyApp");
        assert_eq!(launch_file.processes.len(), 2);
        assert_eq!(launch_file.processes[0].name, "App1");
        assert_eq!(launch_file.processes[0].restart, Some(Restart::Always));
        assert_eq!(launch_file.processes[0].restart_delay_secs, Some(5.0));
    }
}
