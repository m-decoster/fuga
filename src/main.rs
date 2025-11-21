use std::{
    io::{self, stdout},
    panic::{set_hook, take_hook},
};

use clap::Parser;
use ratatui::crossterm::{
    execute,
    terminal::{LeaveAlternateScreen, disable_raw_mode},
};

/// Configuration of applications as TOML files.
mod launch_file;
/// Process spawning.
mod process;
/// Process supervision and management.
mod supervisor;
/// Text-based user interface.
mod tui;



/// Fuga: A monitor for multi-process applications
#[derive(Parser, Debug)]
#[command(name = "Fuga")]
#[command(version)]
#[command(about, long_about = None)]
struct Cli {
    /// Path to the launch file.
    launch_file: String,
}

/// Initialize a panic hook. This allows ratatui to cleanly exit on panic.
pub fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        // Intentionally ignore errors here since we're already in a panic
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

/// Restore the ratatui UI to the standard terminal UI.
pub fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

mod procman;

#[tokio::main()]
async fn main() -> io::Result<()> {
    // let cli = Cli::parse();

    // init_panic_hook();

    // let terminal = ratatui::init();

    // let launch_file = LaunchFile::from_file(&cli.launch_file)?;

    // let mut supervisor = Supervisor::from_launch_file(launch_file).await;

    // let app = App::new(&mut supervisor);
    // let result = app.run(terminal).await;

    // ratatui::restore();

    // result

    procman::run().await;
    Ok(())
}
