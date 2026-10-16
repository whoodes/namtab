//! Private child-side session/controlling-terminal setup for the MK1 broker.
#[cfg(target_os = "linux")]
fn main() {
    if project_x_platform::local_process::run_pty_helper().is_err() {
        std::process::exit(125);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("namtab-pty-helper is available only on the admitted Linux MK1 host");
    std::process::exit(125);
}
