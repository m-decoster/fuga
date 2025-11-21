use futures::future::join_all;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::path::Path;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
    time::sleep,
};

use crate::launch_file::{LaunchFile, ProcessDescription, Restart};
use crate::process::ProcessStatus;

/// A child process.
#[derive(Debug)]
pub struct ChildProcess {
    /// The actual child process.
    child: Arc<Mutex<Option<Child>>>,
    /// A vector containing individual log lines captured from standard output and error.
    log_lines: Arc<Mutex<Vec<String>>>,
    /// The current status of the process.
    status: Arc<Mutex<ProcessStatus>>,
    /// The description from which this process was launched. Used to relaunch the process and display other information.
    description: ProcessDescription,
}

impl ChildProcess {
    /// Returns the process description.
    pub fn description(&self) -> &ProcessDescription {
        &self.description
    }

    /// Returns the process id, or `None` if the process is not running.
    pub async fn id(&self) -> Option<u32> {
        self.child
            .lock()
            .await
            .as_ref()
            .and_then(|child| child.id())
    }

    /// Returns the process status.
    pub async fn status(&self) -> ProcessStatus {
        *self.status.lock().await
    }

    /// Returns the process logs.
    pub async fn logs(&self) -> Vec<String> {
        self.log_lines.lock().await.clone()
    }

    /// Spawns a child process from a process description, setting environment variables (if any).
    /// This will also start several tasks which asynchronously read from the process's standard output and error streams,
    /// and polls the process status every 100 milliseconds.
    ///
    /// # Arguments
    /// - `process_description` - A description of the process to be spawned.
    /// - `env` - A map containing environment variables to be set.
    pub async fn spawn(
        process_description: ProcessDescription,
        env: &HashMap<String, String>,
    ) -> Self {
        let child_result = Command::new(process_description.command.clone())
            .args(process_description.args.clone())
            .envs(env)
            .current_dir(
                process_description
                    .work_dir
                    .clone()
                    .unwrap_or(".".to_string()),
            )
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        if let Some(ref out_file) = process_description.log_file_out {
            let path = Path::new(out_file);
            if let Some(path) = path.parent() {
                match fs::create_dir_all(path).await {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }

        if let Some(ref err_file) = process_description.log_file_err {
            let path = Path::new(err_file);
            if let Some(path) = path.parent() {
                match fs::create_dir_all(path).await {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }

        if let Err(e) = child_result {
            Self {
                child: Arc::new(Mutex::new(None)),
                log_lines: Arc::new(Mutex::new(vec![format!("[ERR] {}", e)])),
                status: Arc::new(Mutex::new(ProcessStatus::Error)),
                description: process_description,
            }
        } else {
            let mut child = child_result.unwrap();

            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();

            let log_lines = Arc::new(Mutex::new(Vec::new()));

            let log_file_out = process_description.log_file_out.clone();
            let log_file_err = process_description.log_file_err.clone();

            // Spawn stdout logger
            {
                let logs = log_lines.clone();
                tokio::spawn(async move {
                    let mut reader = BufReader::new(stdout).lines();

                    let mut out_file = if let Some(out_file_path) = log_file_out {
                        OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(format!("{}.{}", out_file_path, chrono::prelude::Utc::now()))
                            .await
                            .ok()
                    } else {
                        None
                    };

                    while let Ok(Some(line)) = reader.next_line().await {
                        let mut l = logs.lock().await;
                        l.push(format!("[OUT] {}", line));

                        if let Some(ref mut f) = out_file {
                            if let Err(e) = f.write_all(format!("{}\n", line).as_bytes()).await {
                                l.push(format!("[ERR] {}", e));
                            }
                        }
                    }
                });
            }

            // Spawn stderr logger
            {
                let logs = log_lines.clone();
                tokio::spawn(async move {
                    let mut reader = BufReader::new(stderr).lines();

                    let mut err_file = if let Some(err_file_path) = log_file_err {
                        OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(format!("{}.{}", err_file_path, chrono::prelude::Utc::now()))
                            .await
                            .ok()
                    } else {
                        None
                    };

                    while let Ok(Some(line)) = reader.next_line().await {
                        let mut l = logs.lock().await;
                        l.push(format!("[ERR] {}", line));

                        if let Some(ref mut f) = err_file {
                            if let Err(e) = f.write_all(format!("{}\n", line).as_bytes()).await {
                                l.push(format!("[ERR] {}", e));
                            }
                        }
                    }
                });
            }

            let child_arc = Arc::new(Mutex::new(Some(child)));
            let status = Arc::new(Mutex::new(ProcessStatus::Running));

            // Spawn a "waiter" task: polls the child exit without holding the mutex
            {
                let child_clone = child_arc.clone();
                let status_clone = status.clone();
                tokio::spawn(async move {
                    loop {
                        let exit_opt = {
                            let mut guard = child_clone.lock().await;
                            if let Some(child) = guard.as_mut() {
                                match child.try_wait() {
                                    Ok(Some(exit)) => Some(exit.code()),
                                    Ok(None) => None,
                                    Err(_) => Some(None),
                                }
                            } else {
                                Some(None)
                            }
                        };

                        if let Some(code_opt) = exit_opt {
                            *status_clone.lock().await = code_opt.into();
                            break;
                        }

                        sleep(Duration::from_millis(100)).await;
                    }
                });
            }

            Self {
                child: child_arc,
                log_lines,
                description: process_description,
                status,
            }
        }
    }

    /// Kill the process with `SIGTERM`, if it was still running. This sets the process status to `Stopped`.
    pub async fn kill(&mut self) {
        let mut guard = self.child.lock().await;
        if let Some(child) = guard.as_mut()
            && let Some(pid) = child.id()
        {
            if kill(Pid::from_raw(pid.try_into().unwrap()), Signal::SIGTERM).is_ok() {
                *self.status.lock().await = ProcessStatus::Stopped;
            } else {
                *self.status.lock().await = ProcessStatus::Error;
            }
        }
    }

    /// Kill the process (see `Self::kill`) and restart it immediately.
    ///
    /// # Arguments
    /// - `env`: The environment variables to pass to the new process.
    ///
    /// # Returns
    /// The new child process.
    async fn restart(&mut self, env: &HashMap<String, String>) {
        self.kill().await;
        *self = ChildProcess::spawn(self.description.clone(), env).await;
    }
}

/// The process supervisor maintains a list of processes.
pub struct Supervisor {
    /// The common application name.
    name: String,
    /// The child processes.
    children: Vec<ChildProcess>,
    /// The environment variables that are passed to child processes.
    env: HashMap<String, String>,
}

impl Supervisor {
    /// Creates a `Supervisor` from a `LaunchFile`.
    ///
    /// # Arguments
    /// - `launch_file` - A launch file.
    pub async fn from_launch_file(launch_file: LaunchFile) -> Self {
        let env = launch_file.env.unwrap_or_default();
        let mut children = Vec::new();
        for process_description in launch_file.processes {
            children.push(ChildProcess::spawn(process_description, &env).await);
        }

        Self {
            name: launch_file.name,
            children,
            env,
        }
    }

    /// Returns the application's common name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the number of child processes.
    pub fn num_processes(&self) -> usize {
        self.children.len()
    }

    /// Returns an iterator over the children.
    pub fn child_iter(&self) -> impl Iterator<Item = &ChildProcess> {
        self.children.iter()
    }

    /// Returns a reference to a child process.
    ///
    /// # Arguments
    /// - `process_index` - The index of the process.
    ///
    /// # Returns
    /// - `Some(&ChildProcess)` - if the process exists.
    /// - `None` - if `process_index` was out of bounds.
    pub fn child(&self, process_index: usize) -> Option<&ChildProcess> {
        self.children.get(process_index)
    }

    /// Returns a mutable reference to a child process.
    ///
    /// # Arguments
    /// - `process_index` - The index of the process.
    ///
    /// # Returns
    /// - `Some(&mut ChildProcess)` - if the process exists.
    /// - `None` - if `process_index` was out of bounds.
    pub fn child_mut(&mut self, process_index: usize) -> Option<&mut ChildProcess> {
        self.children.get_mut(process_index)
    }

    /// Stops all processes.
    /// This will attempt to stop all processes; then, a result is returned that is `Err` if at least one process failed to stop.
    pub async fn stop_all(&mut self) {
        join_all(self.children.iter_mut().map(|child| child.kill())).await;
    }

    /// Restarts a process.
    pub async fn restart(&mut self, process_index: usize) {
        if let Some(child) = self.children.get_mut(process_index) {
            child.restart(&self.env).await;
        }
    }

    /// Restarts all stopped processes that want to be started.
    pub async fn restart_stopped_processes(&mut self) {
        for child in &mut self.children {
            match &child.description().restart {
                Some(Restart::Always) => match child.status.clone().lock().await.clone() {
                    ProcessStatus::Running => {}
                    _ => {
                        tokio::time::sleep(Duration::from_secs_f32(
                            child.description().restart_delay_secs.unwrap_or_default(),
                        ))
                        .await;

                        child.restart(&self.env).await;
                    }
                },
                Some(Restart::OnFailure) => match child.status.clone().lock().await.clone() {
                    ProcessStatus::Error => {
                        tokio::time::sleep(Duration::from_secs_f32(
                            child.description().restart_delay_secs.unwrap_or_default(),
                        ))
                        .await;

                        child.restart(&self.env).await;
                    }
                    _ => {}
                },
                Some(Restart::Never) => {}
                None => {}
            }
        }
    }
}
