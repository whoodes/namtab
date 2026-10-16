#![cfg(target_os = "linux")]

use std::io::Read;
use std::path::Path;
use std::time::{Duration, Instant};

use project_x_platform::local_process::{
    CommandSpec, Geometry, PtyReader, ReadOutcome, effective_uid, launch_managed, launch_pty,
    peer_uid,
};

const SHORT: Duration = Duration::from_millis(100);
const LIMIT: Duration = Duration::from_secs(3);

#[test]
fn unix_socket_peer_uid_is_kernel_observed_not_client_prose() {
    let (mut sender, mut receiver) = std::os::unix::net::UnixStream::pair().unwrap();
    assert_eq!(peer_uid(&sender).unwrap(), effective_uid());
    assert_eq!(peer_uid(&receiver).unwrap(), effective_uid());
    // Application bytes cannot change the independently obtained identity.
    std::io::Write::write_all(&mut sender, b"declared-role:human;uid:0").unwrap();
    let mut payload = [0; 25];
    receiver.read_exact(&mut payload).unwrap();
    assert_eq!(peer_uid(&receiver).unwrap(), effective_uid());
}

fn shell(script: &str) -> CommandSpec {
    let mut spec = CommandSpec::new("/bin/bash", env!("CARGO_MANIFEST_DIR"));
    spec.args = ["--noprofile", "--norc", "-c", script]
        .map(Into::into)
        .to_vec();
    spec.env = [
        ("PATH".into(), "/usr/bin:/bin".into()),
        ("LC_ALL".into(), "C".into()),
    ]
    .to_vec();
    spec
}

fn helper() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_namtab-pty-helper"))
}

fn read_through(reader: &mut PtyReader, expected: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let started = Instant::now();
    while started.elapsed() < LIMIT {
        let mut buffer = [0_u8; 256];
        match reader.read_chunk(&mut buffer, SHORT).unwrap() {
            ReadOutcome::Data(count) => output.extend_from_slice(&buffer[..count]),
            ReadOutcome::Timeout => (),
            ReadOutcome::Eof => break,
        }
        if output.windows(expected.len()).any(|part| part == expected) {
            return output;
        }
    }
    panic!("did not observe expected synthetic PTY marker; output={output:?}");
}

#[test]
fn real_pty_has_controlling_terminal_merged_stream_and_crlf() {
    let spec = shell(
        "test -t 0 && test -t 1 && test -t 2 || exit 99; printf 'out\\n'; printf 'err\\n' >&2; /usr/bin/stty size; IFS= read -r line",
    );
    let mut child = launch_pty(helper(), &spec, Geometry { cols: 80, rows: 24 }).unwrap();
    let pid = rustix::process::Pid::from_raw(child.process.id() as i32).unwrap();
    // The shell is alive while the marker is read, and its unreaped child PID
    // cannot be reused; these are diagnostic observations, never kill targets.
    assert_eq!(rustix::process::getsid(Some(pid)).unwrap(), pid);
    assert_eq!(rustix::process::getpgid(Some(pid)).unwrap(), pid);
    let output = read_through(&mut child.reader, b"24 80\r\n");
    assert!(output.windows(5).any(|bytes| bytes == b"out\r\n"));
    assert!(output.windows(5).any(|bytes| bytes == b"err\r\n"));
    assert_eq!(child.writer.write_once(b"\n").unwrap(), 1);
    assert!(
        child
            .process
            .wait_timeout(LIMIT)
            .unwrap()
            .unwrap()
            .success()
    );
    let started = Instant::now();
    loop {
        assert!(started.elapsed() < LIMIT, "PTY EOF did not arrive");
        if child.reader.read_chunk(&mut [0; 16], SHORT).unwrap() == ReadOutcome::Eof {
            break;
        }
    }
    assert_eq!(
        child.reader.read_chunk(&mut [0; 16], SHORT).unwrap(),
        ReadOutcome::Eof
    );
}

#[test]
fn resize_and_input_reach_live_pty_without_implicit_retries() {
    let spec = shell(
        "printf 'ready\\n'; IFS= read -r line; printf 'received:%s\\n' \"$line\"; /usr/bin/stty size",
    );
    let mut child = launch_pty(helper(), &spec, Geometry { cols: 80, rows: 24 }).unwrap();
    read_through(&mut child.reader, b"ready\r\n");
    child
        .writer
        .resize(Geometry { cols: 91, rows: 31 })
        .unwrap();
    assert_eq!(
        child.writer.geometry().unwrap(),
        Geometry { cols: 91, rows: 31 }
    );
    assert_eq!(child.writer.write_once(b"marker\n").unwrap(), 7);
    let output = read_through(&mut child.reader, b"31 91\r\n");
    assert!(
        output
            .windows(17)
            .any(|part| part == b"received:marker\r\n")
    );
    assert!(
        child
            .process
            .wait_timeout(LIMIT)
            .unwrap()
            .unwrap()
            .success()
    );
}

#[test]
fn managed_stdout_stderr_exit_and_environment_are_separate() {
    let mut spec =
        shell("printf 'out:%s:%s' \"$MK1_TEST\" \"${HOME-unset}\"; printf 'err' >&2; exit 7");
    spec.env.push(("MK1_TEST".into(), "literal".into()));
    let mut child = launch_managed(&spec).unwrap();
    let mut stdout = String::new();
    let mut stderr = String::new();
    // These fixtures are deliberately much smaller than pipe capacity. A real
    // broker must drain the two pipes concurrently, including after child exit.
    child.stdout.read_to_string(&mut stdout).unwrap();
    child.stderr.read_to_string(&mut stderr).unwrap();
    assert_eq!(stdout, "out:literal:unset");
    assert_eq!(stderr, "err");
    assert_eq!(
        child.process.wait_timeout(LIMIT).unwrap().unwrap().code(),
        Some(7)
    );
}

#[test]
fn launch_errors_and_geometry_are_reported_without_a_live_child() {
    let spec = CommandSpec::new(
        "/namtab-mk1-nonexistent-test-program",
        env!("CARGO_MANIFEST_DIR"),
    );
    assert!(launch_managed(&spec).is_err());
    assert_eq!(
        launch_pty(helper(), &spec, Geometry { cols: 80, rows: 24 })
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    assert!(launch_pty(helper(), &shell("exit 0"), Geometry { cols: 0, rows: 24 }).is_err());
    assert!(
        launch_pty(
            Path::new("/namtab-mk1-nonexistent-helper"),
            &shell("exit 0"),
            Geometry { cols: 80, rows: 24 }
        )
        .is_err()
    );
}

#[test]
fn timeout_and_direct_child_cancellation_are_observable() {
    let spec = shell("printf 'ready\\n'; exec /bin/sleep 30");
    let mut child = launch_pty(helper(), &spec, Geometry { cols: 80, rows: 24 }).unwrap();
    read_through(&mut child.reader, b"ready\r\n");
    assert!(child.reader.read_chunk(&mut [], SHORT).is_err());
    assert!(child.writer.write_once(&[0; 8193]).is_err());
    assert!(child.writer.resize(Geometry { cols: 0, rows: 24 }).is_err());
    assert_eq!(
        child.writer.geometry().unwrap(),
        Geometry { cols: 80, rows: 24 }
    );
    assert_eq!(
        child.reader.read_chunk(&mut [0; 16], SHORT).unwrap(),
        ReadOutcome::Timeout
    );
    assert_eq!(child.process.wait_timeout(Duration::ZERO).unwrap(), None);
    assert!(!child.process.kill_and_wait().unwrap().success());
    assert!(child.process.try_wait().unwrap().is_some());
}

#[test]
fn dropping_handle_reaps_direct_child() {
    let spec = shell("exec /bin/sleep 30");
    let child = launch_managed(&spec).unwrap();
    let pid = rustix::process::Pid::from_raw(child.process.id() as i32).unwrap();
    drop(child);
    // Only the test observes its own already-reaped child identity. There is no
    // API for signalling a reused PID or arbitrary process group.
    assert!(matches!(
        rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::NOHANG),
        Err(rustix::io::Errno::CHILD)
    ));
}
