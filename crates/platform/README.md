# MK1 local process primitives

The Linux-only `local_process` module implements real local PTYs and managed
argv execution. It is an internal platform seam, not a terminal parser, broker,
sandbox or public automation API. The service must authorize every launch,
write, resize and cancellation and impose its session/attempt admission limits.

`CommandSpec` is literal program/argv/cwd plus an **explicit environment**;
the broker environment is cleared. Interactive shells should use an admitted
program and flags such as Bash `--noprofile --norc`, with explicit `PATH`, `TERM`
and locale. This module neither inspects startup files nor invents shell command
boundaries. The caller selects the reviewed helper executable explicitly.

`launch_pty` returns one lifecycle owner, one timed/bounded reader and one
non-cloneable writer/resize handle. PTY stdout/stderr are merged. The master is
nonblocking; writes return the actual accepted prefix without automatic retry.
Resize changes kernel geometry and may cause `SIGWINCH`; it does not prove that
an application has redrawn. Linux master `EIO` after the last slave closes is
reported as EOF. There is no retained output buffer or recovery journal here.

The fresh `namtab-pty-helper` process performs `setsid`, `TIOCSCTTY` and stdio
duplication before replacing itself with the target. All project Rust remains
under `unsafe_code = "forbid"`; no broker-side `pre_exec` callback is used. A
private socket inherited as helper stdin is cloned with CLOEXEC before stdin is
replaced by the slave. The helper writes a fixed startup marker, reports errno
on setup/exec failure and closes the status socket at exec. As with conventional
CLOEXEC launch handshakes, a crash between marker and exec cannot be ruled out
by EOF alone. The startup result is not a process-running guarantee.

`peer_uid` obtains the kernel's Unix-socket peer UID using `SO_PEERCRED`;
`effective_uid` obtains this process's effective UID. The broker can compare
them for the admitted local same-UID boundary without trusting a client field.
This does not attest human identity, differentiate hostile same-UID processes
or implement a sandbox. No stored credentials or account configuration are read.

`launch_managed` gives null stdin and distinct stdout/stderr pipes. Drain them
concurrently, and keep **exit observed**, **stdout EOF** and **stderr EOF**
separate. A descendant can retain an output descriptor after the direct child
exits. There is no global order across the two pipe streams.

`ProcessHandle` uniquely owns `Child`. `kill_and_wait` and drop kill/reap only
that direct child; no arbitrary numeric-PID or group-kill entry point exists.
This prevents a caller from presenting an unrelated PID but is **not descendant
containment**. A shell job may survive cancellation, and external effects may
continue. Closing the last PTY master can cause hangup behavior; that also is
not a universal stop guarantee. Child wait/kill can block in the kernel.

## Dependency and checks

The only new direct dependency is exact **rustix 1.1.5**, cached from crates.io,
with `default-features = false` and features `std`, `pty` (implies `fs`),
`process`, `termios`, `stdio`, `event`, `net`. `stdio` enables safe child-helper
stdio replacement; `event` enables bounded read polling; `net` enables only the
used `SO_PEERCRED` helper (Rustix's feature also enables `linux-raw-sys`'s `net`,
`netlink`, `if_ether` feature definitions). No dependency version is changed.
Public source inspected on 2026-10-16:
`src/pty.rs`, `src/process/{id,ioctl}.rs`, `src/stdio.rs`, `src/io/fcntl.rs`,
`src/fs/fcntl.rs`, `src/termios/tc.rs`, `src/event/poll.rs` and its Linux backend
`PollFd` definition, plus `src/net/{sockopt,types}.rs` and `src/ugid.rs` for
`socket_peercred`, `UCred.uid` and `Uid::as_raw`. Rustix declares
`Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`; this is dependency
provenance, not adoption of a namtab project license.

Primary API references: [PTY module](https://docs.rs/rustix/1.1.5/rustix/pty/index.html),
[controlling terminal](https://docs.rs/rustix/1.1.5/rustix/process/fn.ioctl_tiocsctty.html),
[stdio duplication source](https://docs.rs/crate/rustix/1.1.5/source/src/stdio.rs).

With the workspace lock updated by the integrating change:

```sh
cargo fmt --all --check
cargo test --locked -p project-x-platform
cargo clippy --locked -p project-x-platform --all-targets -- -D warnings
```

Integration tests execute only small synthetic local commands: TTY/session/group
identity, CRLF/merged output, live input/resize/EOF, explicit environment, distinct
pipe attribution, launch errors, timeout and direct-child cleanup. These are
Linux MK1 checks, not shell/TUI compatibility, sandbox, crash-recovery or timing
qualification.
