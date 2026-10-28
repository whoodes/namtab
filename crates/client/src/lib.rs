//! Experimental public clients and contract decoding; no model loop or generic subscriptions.
pub use project_x_protocol::{Context, Intent, Key};
pub mod mcp;
pub mod mk1;
pub use project_x_protocol::mk1 as mk1_protocol;
pub fn validate_request(bytes: &[u8]) -> bool {
    project_x_protocol::decode(bytes).is_ok()
}
#[derive(Default)]
pub struct Cursor {
    committed: u64,
    unknown: bool,
}
impl Cursor {
    pub fn observe(&mut self, next: u64) -> bool {
        if self.unknown || Some(next) != self.committed.checked_add(1) {
            self.unknown = true;
            return false;
        }
        self.committed = next;
        true
    }
    pub fn snapshot(&mut self, at: u64) {
        self.committed = at;
        self.unknown = false;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gaps_require_snapshot() {
        let mut c = Cursor::default();
        assert!(c.observe(1));
        assert!(!c.observe(3));
        assert!(!c.observe(2));
        c.snapshot(5);
        assert!(c.observe(6));
    }
}
