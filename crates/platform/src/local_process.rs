//! Real local Linux execution for the bounded MK1 demo.
//!
//! No shell parsing, terminal state, grants, receipts or persistent process
//! recovery live here. Children receive only the explicitly supplied environment.
//! `Child` stays uniquely owned: cancellation targets that direct child, never a
//! caller-provided numeric PID. It does not promise to stop descendants or undo
//! effects. Waiting for exit does not establish that output has drained.
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec};
use rustix::pty::OpenptFlags;
use rustix::termios::Winsize;

use crate::checked_cells;

pub const MAX_READ_BYTES: usize = 4096;
pub const MAX_WRITE_BYTES: usize = 8192;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(3);
const STARTUP_MARKER: &[u8] = b"NAMTAB_PTY_1\n";

/// Literal program/argv, never an implicit shell command string.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub cwd: PathBuf,
    /// Empty means an empty environment, not inheritance from the broker.
    pub env: Vec<(OsString, OsString)>,
}

impl CommandSpec {
    pub fn new(program: impl AsRef<OsStr>, cwd: impl AsRef<Path>) -> Self {
        Self {
            program: program.as_ref().to_owned(),
            args: Vec::new(),
            cwd: cwd.as_ref().to_owned(),
            env: Vec::new(),
        }
    }

    fn command(&self, program: &OsStr) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(&self.cwd)
            .env_clear()
            .envs(self.env.iter().cloned());
        command
    }
}

/// Rows/columns only; pixel sizes are unknown rather than invented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub cols: u16,
    pub rows: u16,
}

impl Geometry {
    fn winsize(self) -> io::Result<Winsize> {
        if checked_cells(self.cols, self.rows).is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid PTY geometry",
            ));
        }
        Ok(Winsize {
            ws_row: self.rows,
            ws_col: self.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        })
    }
}

/// Sole lifecycle owner. Stream EOF and child exit remain separate observations.
#[derive(Debug)]
pub struct ProcessHandle {
    child: Child,
}

impl ProcessHandle {
    /// Diagnostic identity only; this API never accepts a PID as authority.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    /// Kill only this still-owned direct child, then reap it. This may block in
    /// the kernel; it does not terminate a process group or prove effects stopped.
    pub fn kill_and_wait(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.child.try_wait()? {
            return Ok(status);
        }
        self.child.kill()?;
        self.child.wait()
    }

    pub fn wait_timeout(&mut self, timeout: Duration) -> io::Result<Option<ExitStatus>> {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait()? {
                return Ok(Some(status));
            }
            let remaining = timeout.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Ok(None);
            }
            std::thread::sleep(remaining.min(Duration::from_millis(10)));
        }
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        // Do not detach a direct child accidentally. Descendants are explicitly
        // outside this primitive's cancellation guarantee.
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOutcome {
    Data(usize),
    Timeout,
    Eof,
}

#[derive(Debug)]
pub struct PtyReader {
    file: File,
    eof: bool,
}

impl PtyReader {
    /// At most one bounded chunk; no internal output accumulation.
    pub fn read_chunk(&mut self, buffer: &mut [u8], timeout: Duration) -> io::Result<ReadOutcome> {
        if buffer.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "empty PTY read buffer",
            ));
        }
        if self.eof {
            return Ok(ReadOutcome::Eof);
        }
        if !ready(&self.file, PollFlags::IN, timeout)? {
            return Ok(ReadOutcome::Timeout);
        }
        let limit = buffer.len().min(MAX_READ_BYTES);
        match self.file.read(&mut buffer[..limit]) {
            Ok(0) => {
                self.eof = true;
                Ok(ReadOutcome::Eof)
            }
            Ok(count) => Ok(ReadOutcome::Data(count)),
            // Linux PTY masters report EIO once the last slave closes.
            Err(error) if error.raw_os_error() == Some(rustix::io::Errno::IO.raw_os_error()) => {
                self.eof = true;
                Ok(ReadOutcome::Eof)
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(ReadOutcome::Timeout),
            Err(error) => Err(error),
        }
    }
}

#[derive(Debug)]
pub struct PtyWriter {
    file: File,
}

impl PtyWriter {
    /// One nonblocking OS write, with its exact accepted prefix returned. Caller
    /// must account for partial writes; this never retries non-idempotent input.
    pub fn write_once(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_WRITE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PTY input exceeds chunk limit",
            ));
        }
        self.file.write(bytes)
    }

    /// TIOCSWINSZ can notify the foreground process group with SIGWINCH. It is
    /// not an application acknowledgement that layout completed.
    pub fn resize(&self, geometry: Geometry) -> io::Result<()> {
        rustix::termios::tcsetwinsize(&self.file, geometry.winsize()?).map_err(Into::into)
    }

    pub fn geometry(&self) -> io::Result<Geometry> {
        let size = rustix::termios::tcgetwinsize(&self.file)?;
        Ok(Geometry {
            cols: size.ws_col,
            rows: size.ws_row,
        })
    }
}

#[derive(Debug)]
pub struct PtyProcess {
    pub process: ProcessHandle,
    /// stdout and stderr are merged by the PTY; no attribution is invented.
    pub reader: PtyReader,
    pub writer: PtyWriter,
}

/// Launch through the dedicated, reviewed helper, without unsafe pre-exec code.
/// The helper installs a session/controlling terminal before executing argv.
/// A CLOEXEC status socket reports setup/exec errors; EOF after its marker is
/// the conventional exec-boundary indication, not proof against a helper crash
/// in the tiny marker-to-exec window. Later process exit remains authoritative.
pub fn launch_pty(helper: &Path, spec: &CommandSpec, geometry: Geometry) -> io::Result<PtyProcess> {
    let size = geometry.winsize()?;
    let master =
        rustix::pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC)?;
    rustix::pty::grantpt(&master)?;
    rustix::pty::unlockpt(&master)?;
    let slave = rustix::pty::ioctl_tiocgptpeer(
        &master,
        OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC,
    )?;
    rustix::termios::tcsetwinsize(&slave, size)?;
    let (mut status_parent, status_child) = UnixStream::pair()?;
    status_parent.set_read_timeout(Some(STARTUP_TIMEOUT))?;
    let mut command = spec.command(helper.as_os_str());
    command.arg("--").arg(&spec.program).args(&spec.args);
    let status_child_fd: OwnedFd = status_child.into();
    command.stdin(Stdio::from(status_child_fd));
    command.stdout(Stdio::from(slave.try_clone()?));
    command.stderr(Stdio::from(slave));
    let mut process = ProcessHandle {
        child: command.spawn()?,
    };
    // Command retains its configured Stdio handles; close the parent copies so
    // they cannot suppress EOF on either the status socket or PTY slave.
    drop(command);
    let mut status = Vec::new();
    let startup = (&mut status_parent).take(256).read_to_end(&mut status);
    if let Err(error) = startup {
        let _ = process.kill_and_wait();
        return Err(io::Error::new(
            error.kind(),
            "PTY helper startup timed out or disconnected",
        ));
    }
    if status != STARTUP_MARKER {
        let _ = process.kill_and_wait();
        let error = status
            .strip_prefix(STARTUP_MARKER)
            .unwrap_or(&status)
            .strip_prefix(b"ERROR ")
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|value| value.trim().parse::<i32>().ok())
            .filter(|errno| *errno > 0)
            .map(io::Error::from_raw_os_error)
            .unwrap_or_else(|| io::Error::other("PTY helper setup or exec failed"));
        return Err(error);
    }
    let flags = rustix::fs::fcntl_getfl(&master)?;
    rustix::fs::fcntl_setfl(&master, flags | rustix::fs::OFlags::NONBLOCK)?;
    let master_file = File::from(master);
    let writer = PtyWriter {
        file: master_file.try_clone()?,
    };
    Ok(PtyProcess {
        process,
        reader: PtyReader {
            file: master_file,
            eof: false,
        },
        writer,
    })
}

#[derive(Debug)]
pub struct ManagedProcess {
    pub process: ProcessHandle,
    pub stdout: ChildStdout,
    pub stderr: ChildStderr,
}

/// Managed argv uses null stdin and two distinct pipes. Output must be drained
/// concurrently by the caller; no platform buffer or inferred command boundary.
pub fn launch_managed(spec: &CommandSpec) -> io::Result<ManagedProcess> {
    let mut command = spec.command(&spec.program);
    command
        .args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A separate group is useful for job identity, but cancellation deliberately
    // targets only the live owned child; group-wide termination is not claimed.
    command.process_group(0);
    let mut process = ProcessHandle {
        child: command.spawn()?,
    };
    let stdout = process
        .child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing stdout pipe"))?;
    let stderr = process
        .child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("missing stderr pipe"))?;
    Ok(ManagedProcess {
        process,
        stdout,
        stderr,
    })
}

fn ready(file: &File, flags: PollFlags, timeout: Duration) -> io::Result<bool> {
    let seconds = i64::try_from(timeout.as_secs())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "poll timeout too large"))?;
    let timespec = Timespec {
        tv_sec: seconds,
        tv_nsec: i64::from(timeout.subsec_nanos()),
    };
    let mut descriptors = [PollFd::new(file, flags)];
    Ok(rustix::event::poll(&mut descriptors, Some(&timespec))? > 0)
}

/// Entry point for the single-purpose helper binary. Public to avoid duplicating
/// this reviewed setup seam; not a broker or user-facing execution protocol.
pub fn run_pty_helper() -> io::Result<()> {
    let status_fd = rustix::io::fcntl_dupfd_cloexec(rustix::stdio::stdin(), 3)?;
    let mut status = UnixStream::from(status_fd);
    let result = prepare_and_exec(&mut status);
    if let Err(error) = &result {
        // Bounded machine status, never argv, environment or terminal content.
        let errno = error.raw_os_error().unwrap_or(0);
        let _ = writeln!(status, "ERROR {errno}");
    }
    result
}

fn prepare_and_exec(status: &mut UnixStream) -> io::Result<()> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new("--")) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "helper requires argv separator",
        ));
    }
    let program = args
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "helper requires a program"))?;
    rustix::process::setsid()?;
    rustix::process::ioctl_tiocsctty(rustix::stdio::stdout())?;
    rustix::stdio::dup2_stdin(rustix::stdio::stdout())?;
    status.write_all(STARTUP_MARKER)?;
    // This helper is a fresh single-threaded process. Safe CommandExt::exec
    // replaces it directly; there is no closure running after fork in a broker.
    Err(Command::new(program).args(args).exec())
}
