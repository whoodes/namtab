//! Linux MK1 process and PTY primitives, without terminal or dispatch authority.
//!
//! These are local developer-host processes, not a sandbox. The caller owns
//! authorization, capture policy, command limits and terminal interpretation.
//! A PTY merges stdout/stderr; managed pipes retain their separate identity.
#[cfg(target_os = "linux")]
pub mod local_process;
use project_x_domain::Context;
pub const MAX_CELLS: u32 = 80_000;
pub fn checked_cells(cols: u16, rows: u16) -> Option<usize> {
    let cells = u32::from(cols).checked_mul(u32::from(rows))?;
    (cols > 0 && rows > 0 && cells <= MAX_CELLS).then_some(cells as usize)
}
/// Canonical context must be known before any platform operation can be planned.
pub fn validate_view(context: &Context, cols: u16, rows: u16) -> bool {
    context.geometry > 0 && checked_cells(cols, rows).is_some()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_geometry() {
        assert_eq!(checked_cells(80, 24), Some(1920));
        assert_eq!(checked_cells(0, 10), None);
        assert_eq!(checked_cells(u16::MAX, u16::MAX), None);
    }
}
