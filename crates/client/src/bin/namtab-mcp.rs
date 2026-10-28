//! Local stdio MCP bridge. It never loads provider configuration or credentials.
use project_x_client::mcp::{self, Bridge};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.len() != 4 || a[0] != "--socket" || a[2] != "--epoch" {
        return Err("usage: namtab-mcp --socket EXPLICIT_PATH --epoch EXPECTED_EPOCH".into());
    }
    let epoch = a[3].to_str().ok_or("epoch must be UTF8")?;
    let mut bridge = Bridge::connect(std::path::Path::new(&a[1]), epoch)?;
    mcp::serve(
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
        &mut bridge,
    )?;
    Ok(())
}
