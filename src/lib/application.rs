/// Representation of an Application to be launched.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Application {
    /// Name of the application.
    pub name: String,
    /// List of processes to be launched.
    pub processes: Vec<Process>,
}

impl Application {
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

    /// Parse a launch file from a file path.
    ///
    /// Args:
    ///    file_path: The path to the launch file.
    ///
    /// Returns:
    ///     Ok(Application) if parsing is successful, Err(String) with an error otherwise.
    pub fn from_file(file_path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read launch file: {}", e))?;
        Self::from_str(&content)
            .map_err(|e| format!("Failed to parse launch file: {}", e))
    }

    pub fn launch_all(&self) -> Result<(), String> {
        // Placeholder for actual launching logic.
        println!("Launching all applications for '{}'", self.name);
        for process in &self.processes {
            println!("Launching application: {}", process.name);
            // Here you would add the logic to actually launch the process.
        }
        Ok(())
    }
}

/// Representation of a process to be launched.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Process {
    /// A unique name for the process.
    pub name: String,
    /// Command to start the process.
    pub command: String,
    /// Arguments to pass to the process.
    pub args: Vec<String>,
    /// Whether the process should be restarted. If None, defaults to Restart::Never.
    pub restart: Option<Restart>,
    /// How long to wait before restarting the process. If None, defaults to 0 (immediate restart).
    pub restart_delay_secs: Option<f32>,
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