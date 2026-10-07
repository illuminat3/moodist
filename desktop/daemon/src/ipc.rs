//! How the UI and the daemon find and talk to each other. Shared by both binaries
//! (the UI includes this file with `#[path]`).
//!
//! Linux uses a Unix socket in `$XDG_RUNTIME_DIR`. Windows' standard library has no
//! Unix sockets, so there the daemon listens on a random loopback TCP port and
//! publishes it in `%LOCALAPPDATA%\Moodist\daemon.port`.

use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[cfg(windows)]
pub use std::net::{TcpListener as Listener, TcpStream as Stream};
#[cfg(unix)]
pub use std::os::unix::net::{UnixListener as Listener, UnixStream as Stream};

#[cfg(unix)]
fn endpoint() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|p| p.is_absolute()) {
        Some(dir) => dir.join("moodist.sock"),
        None => std::env::temp_dir().join(format!("moodist-{}.sock", std::env::var("USER").unwrap_or_default())),
    }
}

#[cfg(windows)]
fn endpoint() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Moodist").join("daemon.port")
}

pub fn describe() -> String {
    endpoint().display().to_string()
}

/// Connects to a running daemon.
pub fn connect() -> io::Result<Stream> {
    #[cfg(unix)]
    return Stream::connect(endpoint());
    #[cfg(windows)]
    {
        let port: u16 = std::fs::read_to_string(endpoint())?
            .trim()
            .parse()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
        let stream = Stream::connect(("127.0.0.1", port))?;
        stream.set_nodelay(true)?;
        Ok(stream)
    }
}

/// Claims the endpoint. Call only after `connect` failed, i.e. no daemon is running.
pub fn bind() -> io::Result<Listener> {
    #[cfg(unix)]
    {
        let path = endpoint();
        let _ = std::fs::remove_file(&path); // stale socket from a crash
        Listener::bind(path)
    }
    #[cfg(windows)]
    {
        let listener = Listener::bind(("127.0.0.1", 0))?;
        let path = endpoint();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, listener.local_addr()?.port().to_string())?;
        Ok(listener)
    }
}

/// Accepted connections, set up for small interactive messages.
pub fn incoming(listener: &Listener) -> impl Iterator<Item = Stream> + '_ {
    listener.incoming().flatten().inspect(|_s| {
        #[cfg(windows)]
        let _ = _s.set_nodelay(true);
    })
}

pub fn cleanup() {
    let _ = std::fs::remove_file(endpoint());
}

/// The other Moodist binary installed next to this one, or a bare name for `$PATH`.
pub fn sibling(name: &str) -> PathBuf {
    let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    std::env::current_exe()
        .ok()
        .and_then(|p| Some(p.parent()?.join(&file)))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from(file))
}

/// A command whose process outlives its parent and the terminal it was started from.
pub fn detached(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    cmd
}
