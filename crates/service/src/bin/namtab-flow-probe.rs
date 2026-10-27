//! Isolated synthetic workflow evidence; never attaches to a live daemon.
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let artifacts = PathBuf::from(
        args.next()
            .ok_or("usage: namtab-flow-probe FRESH-ABS-DIRECTORY ABS-FIXTURE [normal|takeover|nonzero|overflow]")?,
    );
    let fixture = PathBuf::from(args.next().ok_or("absolute fixture path required")?);
    let mode = args
        .next()
        .map(|s| s.into_string())
        .transpose()
        .map_err(|_| "invalid_mode_encoding")?
        .unwrap_or_else(|| "normal".into());
    if args.next().is_some() || !artifacts.is_absolute() {
        return Err("invalid_arguments".into());
    }
    let report = project_x_service::flow::run(&artifacts, &fixture, &mode)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
