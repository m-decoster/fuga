use std::collections::HashMap;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

type Pid = i32;
type ProcessName = String;
type LogLine = String;

#[derive(Debug)]
enum ProcessState {
    Running,
    Success,
    Error,
    Terminated,
    Killed,
}

struct Process {
    name: String,
    proc: Child,
}

impl Process {
    pub fn spawn(name: String, event_tx: mpsc::Sender<ProcessEvent>) -> std::io::Result<()> {
        let mut proc = if name.contains("ping") {
            Command::new("ping")
                .arg("google.com")
                .stdout(std::process::Stdio::piped())
                .spawn()?
        } else {
            Command::new("echo")
                .arg("this process will immediately exit with status success")
                .stdout(std::process::Stdio::piped())
                .spawn()?
        };

        let stdout = proc.stdout.take().unwrap();

        let cloned_name = name.clone();
        let cloned_tx = event_tx.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = cloned_tx
                    .send(ProcessEvent::LogLine(cloned_name.clone(), line))
                    .await;
            }
        });

        let cloned_name = name.clone();

        tokio::spawn(async move {
            let status = proc.wait().await.unwrap();
            if status.success() {
                let _ = event_tx
                    .send(ProcessEvent::StateChanged(
                        cloned_name.clone(),
                        ProcessState::Success,
                    ))
                    .await;
            } else {
                let _ = event_tx
                    .send(ProcessEvent::StateChanged(
                        cloned_name.clone(),
                        ProcessState::Error,
                    ))
                    .await;
            }
        });

        Ok(())
    }
}

enum SupervisorCommand {
    StopProcess,
    StartProcess,
}

enum SupervisorEvent {
    LogLine(ProcessName, LogLine),
    StateChanged(ProcessName, ProcessState),
}

type ProcessEvent = SupervisorEvent;

struct Supervisor {
    command_rx: mpsc::Receiver<SupervisorCommand>,
    proc_event_rx: mpsc::Receiver<ProcessEvent>,
    sup_event_tx: mpsc::Sender<SupervisorEvent>,
}

impl Supervisor {
    fn new(
        command_rx: mpsc::Receiver<SupervisorCommand>,
        sup_event_tx: mpsc::Sender<SupervisorEvent>,
    ) -> std::io::Result<Supervisor> {
        let (proc_event_tx, proc_event_rx) = mpsc::channel(100);

        Process::spawn("ping".into(), proc_event_tx.clone())?;
        Process::spawn("echo".into(), proc_event_tx)?;

        Ok(Self {
            command_rx,
            proc_event_rx,
            sup_event_tx,
        })
    }

    fn spawn_tasks(mut self) {
        tokio::spawn(async move {
            while let Some(cmd) = self.proc_event_rx.recv().await {
                match cmd {
                    ProcessEvent::LogLine(n, l) => self
                        .sup_event_tx
                        .send(SupervisorEvent::LogLine(
                            n.clone(),
                            format!("LOG: {} -- {}", n, l),
                        ))
                        .await
                        .unwrap(),
                    ProcessEvent::StateChanged(n, s) => self
                        .sup_event_tx
                        .send(SupervisorEvent::StateChanged(n.clone(), s))
                        .await
                        .unwrap(),
                }
            }
        });

        tokio::spawn(async move {
            while let Some(cmd) = self.command_rx.recv().await {
                match cmd {
                    SupervisorCommand::StopProcess => todo!(),
                    SupervisorCommand::StartProcess => {
                        println!("Received start process command");
                    }
                }
            }
        });
    }
}

pub async fn run() -> std::io::Result<()> {
    let (command_tx, command_rx) = mpsc::channel(100);
    let (event_tx, mut event_rx) = mpsc::channel(100);

    let supervisor = Supervisor::new(command_rx, event_tx)?;

    command_tx
        .send(SupervisorCommand::StartProcess)
        .await
        .unwrap();

    supervisor.spawn_tasks();

    // We'd run this, but also have channel communication between this function and the TUI run function...
    while let Some(cmd) = event_rx.recv().await {
        match cmd {
            SupervisorEvent::LogLine(_, l) => println!("from supervisor to main: {}", l),
            SupervisorEvent::StateChanged(n, process_state) => {
                println!("Process {} state is now {:?}", n, process_state)
            }
        }
    }

    Ok(())
}
