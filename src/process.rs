use std::{collections::HashMap, path::Path};

use tokio::{
    fs::{self, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::mpsc,
};

use crate::launch_file::ProcessDescription;

/// The current status of a child process.
#[derive(Debug, Clone, Copy, Default)]
pub enum ProcessStatus {
    /// The process has not yet been started.
    #[default]
    Idle,
    /// The process is currently running.
    Running,
    /// The process was stopped by the user (through the UI).
    Stopped,
    /// The process exited with a zero exit code.
    Success,
    /// The process exited with a non-zero exit code.
    Error,
}

impl From<Option<i32>> for ProcessStatus {
    fn from(code: Option<i32>) -> Self {
        match code {
            Some(0) => ProcessStatus::Success,
            Some(_) => ProcessStatus::Error,
            None => ProcessStatus::Stopped,
        }
    }
}

/// An error thrown when spawning a process.
#[derive(Debug)]
pub struct ProcessSpawningError {
    pub message: String,
}

impl ProcessSpawningError {
    /// Throw a `ProcessSpawningError` for failing to capture standard output.
    pub fn no_stdout() -> Self {
        ProcessSpawningError {
            message: "Failed to capture stdout".to_string(),
        }
    }

    /// Throw a `ProcessSpawningError` for failing to capture standard error.
    pub fn no_stderr() -> Self {
        ProcessSpawningError {
            message: "Failed to capture stderr".to_string(),
        }
    }
}

impl std::fmt::Display for ProcessSpawningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProcessSpawningError: {}", self.message)
    }
}

impl std::error::Error for ProcessSpawningError {}

impl From<std::io::Error> for ProcessSpawningError {
    fn from(error: std::io::Error) -> Self {
        ProcessSpawningError {
            message: error.to_string(),
        }
    }
}

/// A log file.
struct LogFile {
    file: tokio::fs::File,
}

impl LogFile {
    /// Creates a new log file.
    ///
    /// # Arguments
    /// - `log_file_path` - The path to the log file.
    async fn new(log_file_path: String) -> std::io::Result<Self> {
        // Ensure directory exists.
        if let Some(parent) = Path::new(&log_file_path).parent() {
            fs::create_dir_all(parent).await?;
        }

        // Open file with timestamp.
        let timestamp = chrono::prelude::Utc::now();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("{}.{}", log_file_path, timestamp))
            .await?;

        Ok(Self { file })
    }

    /// Writes a line to the log file.
    ///
    /// # Arguments
    /// - `line`: The line to write.
    async fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        self.file
            .write_all(format!("{}\n", line).as_bytes())
            .await?;
        Ok(())
    }
}

/// Internal message type for communicating between stream readers and collector
enum LogMessage {
    /// A standard output message.
    Stdout(String),
    /// A standard error message.
    Stderr(String),
}

impl std::fmt::Display for LogMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogMessage::Stdout(line) => write!(f, "[OUT] {}", line),
            LogMessage::Stderr(line) => write!(f, "[ERR] {}", line),
        }
    }
}

/// Represents a spawned process.
pub struct Process {
    /// The logged lines.
    log_lines: Vec<String>,
    /// The process's status.
    status: ProcessStatus,
}

impl Process {
    /// Returns the log contents.
    pub fn log_lines(&self) -> &[String] {
        &self.log_lines
    }

    /// Returns the current status.
    pub fn status(&self) -> ProcessStatus {
        self.status
    }

    /// Spawns a task to read from a stream and send messages through a channel.
    /// Returns an optional `LogFile` for writing received messages to disk.
    async fn spawn_stream_reader<F>(
        stream: impl tokio::io::AsyncRead + Unpin + Send + 'static,
        log_file_path: Option<String>,
        tx: mpsc::Sender<LogMessage>,
        msg_constructor: F,
    ) -> std::io::Result<Option<LogFile>>
    where
        F: Fn(String) -> LogMessage + Send + 'static,
    {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stream).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx.send(msg_constructor(line)).await;
            }
        });

        match log_file_path {
            Some(p) => Ok(Some(LogFile::new(p).await?)),
            None => Ok(None),
        }
    }

    pub async fn spawn(
        process_description: ProcessDescription,
        env: &HashMap<String, String>,
    ) -> Result<Self, ProcessSpawningError> {
        let mut child = Command::new(process_description.command)
            .args(process_description.args)
            .envs(env)
            .current_dir(process_description.work_dir.unwrap_or(".".to_string()))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(ProcessSpawningError::no_stdout)?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(ProcessSpawningError::no_stderr)?;

        let mut log_lines = Vec::new();

        // Create channel for log messages
        let (log_tx, mut log_rx) = mpsc::channel::<LogMessage>(100);

        // Spawn stream readers and get log files
        let mut out_log_file = Self::spawn_stream_reader(
            stdout,
            process_description.log_file_out,
            log_tx.clone(),
            LogMessage::Stdout,
        )
        .await?;
        let mut err_log_file = Self::spawn_stream_reader(
            stderr,
            process_description.log_file_err,
            log_tx,
            LogMessage::Stderr,
        )
        .await?;

        while let Some(msg) = log_rx.recv().await {
            // Use Display implementation for formatting
            log_lines.push(msg.to_string());

            // Write to appropriate log file
            let result = match &msg {
                LogMessage::Stdout(line) => {
                    if let Some(ref mut l) = out_log_file {
                        l.write_line(line).await
                    } else {
                        Ok(())
                    }
                }
                LogMessage::Stderr(line) => {
                    if let Some(ref mut l) = err_log_file {
                        l.write_line(line).await
                    } else {
                        Ok(())
                    }
                }
            };

            if let Err(e) = result {
                log_lines.push(format!("[ERR] Failed to write to log: {}", e));
            }
        }

        // Channel closed - both reader tasks finished
        // Now wait for the child process to exit and capture exit code
        let exit_status = child.wait().await;
        let status = exit_status.ok().and_then(|s| s.code()).into();

        Ok(Self { log_lines, status })
    }
}
