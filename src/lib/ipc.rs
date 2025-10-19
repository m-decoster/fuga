//! Inter-process communication utilities, including data structures.

use std::io;
use std::mem::size_of;

use bincode;
use tokio::io::Interest;
use tokio::net::UnixStream;

use crate::application::Application;

/// Interface between client and demo.
pub struct DaemonApi {
    /// UnixStream for communication.
    stream: UnixStream
}

impl DaemonApi {
    /// Create a new instance, connecting to the given socket path.
    pub fn try_with_socket_path(socket_path: &str) -> io::Result<Self> {
        let mut stream = UnixStream::connect(socket_path)?;

        Ok(
            Self {
                stream
            }
        )
    }
}

impl DaemonApi {
    /// Send a `StartMessage`.
    pub async fn send_start_application(&mut self, application: Application) -> io::Result<()> {
        let ready = self.stream.ready(Interest::WRITABLE).await?;
        if ready.is_writable() {
            let bytes = bincode::serialize(&application)?;
            let length_bytes = bytes.len().to_le_bytes();
            self.stream.write_all(&length_bytes)?;

            self.stream.write_all(&bytes)?;
        }

        // TODO: stream responses to the client...

        Ok(())
    }
}
