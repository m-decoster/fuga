use clap::{Parser, Subcommand};
use fuga::application::Application;

/// Fuga: An orchestrator for multi-process applications
#[derive(Parser, Debug)]
#[command(name = "Fuga")]
#[command(version)]
#[command(about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available commands for Fuga.
#[derive(Debug)]
#[derive(Subcommand)]
enum Commands {
    /// Start an application.
    Start {
        /// Path to the application file.
        application: String
    },
    /// Get the status of an application.
    Status {
        /// Application name.
        name: String
    },
    /// Stop an application.
    Stop {
        /// Application name.
        name: String
    },
    /// Manage a running application.
    Application {
        /// Application name.
        name: String,

        #[command(subcommand)]
        action: ApplicationCommand
    }
}

/// Available commands for Fuga-managed applications.
#[derive(Debug)]
#[derive(Subcommand)]
enum ApplicationCommand {
    /// Restart a sub-process.
    Restart {
        /// Name of the process as defined in the launch file.
        process_name: String
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Start { application } => {
            let application = Application::from_file(application);
            if let Ok(lf) = application {
                println!("Launching application: {}", lf.name());
                // TODO: send application to daemon.
            } else {
                eprintln!("Failed to read launch file: {}", application.err().unwrap());
            }
        },
        Commands::Status { name } => {
            // TODO: communicate with daemon to get status.
            println!("'myapp status' was used, name is: {name:?}");
        },
        Commands::Stop { name } => {
            // TODO: communicate with daemon to stop application, killing all its processes.
            println!("'myapp stop' was used, name is: {name:?}");
        },
        Commands::Application { name, action } => {
            match action {
                ApplicationCommand::Restart { process_name } => {
                    println!("'myapp application restart' was used, name is: {name:?}, process_name is: {process_name:?}");
                }
            }
        }
    }
}
