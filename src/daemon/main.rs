use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
};
use nix::unistd::{chown, Gid, Uid};
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Path to the socket file for communication between clients and the daemon.
const SOCKET_PATH: &str = "/run/fuga/fuga.sock";
/// Directory where the daemon runtime files are stored.
const RUN_DIR: &str = "/run/fuga";

/// Handle a client connection.
///
/// Args:
///    stream: The UnixStream representing the client connection.
///
/// Returns:
///    A Result indicating success or failure.
async fn handle_client(mut stream: UnixStream) -> std::io::Result<()> {
    let mut buf = vec![0; 1024];
    let n = stream.read(&mut buf).await?;
    let cmd = String::from_utf8_lossy(&buf[..n]);
    println!("Received: {}", cmd);

    // respond with a placeholder JSON
    stream.write_all(b"{\"status\": \"ok\"}\n").await?;
    Ok(())
}

/// Create the /run/fugad directory with appropriate permissions.
///
/// Returns:
///    A Result indicating success or failure.
fn setup_run_dir() -> std::io::Result<()> {
    // Create /run/fugad if needed
    if !Path::new(RUN_DIR).exists() {
        fs::create_dir_all(RUN_DIR)?;
        // Permissions: 0750 → owner & group can access, others denied
        fs::set_permissions(RUN_DIR, fs::Permissions::from_mode(0o750))?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    setup_run_dir()?;

    if Path::new(SOCKET_PATH).exists() {
        fs::remove_file(SOCKET_PATH)?;
    }

    let listener = UnixListener::bind(SOCKET_PATH)?;
    // Restrict socket access to owner only
    fs::set_permissions(SOCKET_PATH, fs::Permissions::from_mode(0o600))?;

    println!("Supervisor daemon listening on {}", SOCKET_PATH);

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(handle_client(stream));
    }
}
