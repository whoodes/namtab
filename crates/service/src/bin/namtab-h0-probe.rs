//! Bounded, fresh-resource H0 experiment. No listening socket or live attachment.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{collections::BTreeMap, path::PathBuf};
    let mut args = std::env::args_os().skip(1);
    let mut values = BTreeMap::new();
    while let Some(key) = args.next() {
        let value = args.next().ok_or("each option needs a path")?;
        if values.insert(key, PathBuf::from(value)).is_some() {
            return Err("duplicate_option".into());
        }
    }
    let mut take = |key: &str| {
        values
            .remove(std::ffi::OsStr::new(key))
            .ok_or("required: --artifacts --worker --pty-helper")
    };
    let config = project_x_service::h0_write::Config {
        artifacts: take("--artifacts")?,
        worker: take("--worker")?,
        pty_helper: take("--pty-helper")?,
    };
    if !values.is_empty() {
        return Err("unknown_option".into());
    }
    let report = project_x_service::h0_write::run(config)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
