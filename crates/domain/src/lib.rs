//! In-memory fencing/receipt model. No durable or operating-system guarantees.
use std::collections::BTreeMap;
#[derive(Clone, PartialEq, Eq)]
pub struct Context {
    pub session: String,
    pub incarnation: String,
    pub endpoint: String,
    pub generation: u64,
    pub prefix: u64,
    pub geometry: u64,
    pub profile: u64,
    pub focus: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    Reserved,
    Zero,
    Partial,
    Full,
    Unknown,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Admission {
    Reserved,
    Existing(Delivery),
    Stale,
    Capacity,
    IdConflict,
}
pub struct Ledger {
    current: Context,
    entries: BTreeMap<String, ([u8; 32], Delivery)>,
    limit: usize,
}
impl Ledger {
    pub fn new(current: Context, limit: usize) -> Self {
        Self {
            current,
            entries: BTreeMap::new(),
            limit,
        }
    }
    pub fn set_context(&mut self, current: Context) {
        self.current = current;
    }
    pub fn reserve(&mut self, id: &str, binding: [u8; 32], context: &Context) -> Admission {
        // No reuse or retry even when context changes. This prototype has no expiry/pruning.
        if let Some((original, outcome)) = self.entries.get(id) {
            return if *original == binding {
                Admission::Existing(*outcome)
            } else {
                Admission::IdConflict
            };
        }
        if context != &self.current {
            return Admission::Stale;
        }
        if self.entries.len() >= self.limit {
            return Admission::Capacity;
        }
        self.entries
            .insert(id.to_owned(), (binding, Delivery::Reserved));
        Admission::Reserved
    }
    pub fn finish(&mut self, id: &str, outcome: Delivery) -> bool {
        if outcome == Delivery::Reserved {
            return false;
        }
        match self.entries.get_mut(id) {
            Some((_, v)) if *v == Delivery::Reserved => {
                *v = outcome;
                true
            }
            _ => false,
        }
    }
    pub fn mark_transport_lost(&mut self) {
        for (_, v) in self.entries.values_mut() {
            if *v == Delivery::Reserved {
                *v = Delivery::Unknown;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn ctx(generation: u64) -> Context {
        Context {
            session: "s".into(),
            incarnation: "i".into(),
            endpoint: "c".into(),
            generation,
            prefix: 1,
            geometry: 1,
            profile: 1,
            focus: 1,
        }
    }
    #[test]
    fn unknown_never_becomes_retry() {
        let mut l = Ledger::new(ctx(1), 2);
        assert_eq!(l.reserve("a", [0; 32], &ctx(1)), Admission::Reserved);
        l.mark_transport_lost();
        l.set_context(ctx(2));
        assert_eq!(
            l.reserve("a", [0; 32], &ctx(2)),
            Admission::Existing(Delivery::Unknown)
        );
        assert!(!l.finish("a", Delivery::Full));
        assert_eq!(l.reserve("b", [0; 32], &ctx(1)), Admission::Stale);
    }
    #[test]
    fn reused_id_with_changed_payload_is_conflict() {
        let mut l = Ledger::new(ctx(1), 2);
        assert_eq!(l.reserve("a", [0; 32], &ctx(1)), Admission::Reserved);
        assert_eq!(l.reserve("a", [1; 32], &ctx(1)), Admission::IdConflict);
    }
    #[test]
    fn full_storage_does_not_evict_receipts() {
        let mut l = Ledger::new(ctx(1), 1);
        l.reserve("a", [0; 32], &ctx(1));
        assert_eq!(l.reserve("b", [0; 32], &ctx(1)), Admission::Capacity);
    }
    proptest! {
     #[test] fn old_generations_never_reserve(g in 1u64..u64::MAX) { let mut l=Ledger::new(ctx(g),1);prop_assert_eq!(l.reserve("a", [0;32], &ctx(g-1)),Admission::Stale); }
     #[test] fn at_most_one_reservation_per_id(trace in prop::collection::vec(0u8..4,0..100)) {
      let mut l=Ledger::new(ctx(1),8);let mut reservations=0;
      for action in trace {match action {0=>{if l.reserve("a", [0;32], &ctx(1))==Admission::Reserved{reservations+=1}},1=>{l.finish("a",Delivery::Partial);},2=>l.mark_transport_lost(),_=>l.set_context(ctx(2))}}
      prop_assert!(reservations<=1);
     }
    }
}
