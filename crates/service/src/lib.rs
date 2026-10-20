//! Offline admission helpers and a separately admitted local H0 experiment.
//! Neither is a product native API or durable dispatcher.
/// Bounded managed producer-consumer proof; no production scheduler or transport.
pub mod flow;
pub mod h0_write;
#[path = "mk1/worker.rs"]
mod terminal_worker;
use project_x_domain::{Admission, Context, Ledger};
use project_x_protocol::{DecodeError, InputRequest};
use sha2::{Digest, Sha256};
pub fn context(request: &InputRequest) -> Result<Context, DecodeError> {
    project_x_protocol::validate(request)?;
    let c = &request.context;
    let number = |s: &str| s.parse::<u64>().map_err(|_| DecodeError::Decimal);
    Ok(Context {
        session: c.session.clone(),
        incarnation: c.incarnation.clone(),
        endpoint: c.endpoint.clone(),
        generation: number(&c.generation)?,
        prefix: number(&c.prefix)?,
        geometry: number(&c.geometry)?,
        profile: number(&c.profile)?,
        focus: number(&c.focus)?,
    })
}
pub fn reserve_offline(
    ledger: &mut Ledger,
    request: &InputRequest,
    cols: u16,
    rows: u16,
) -> Result<Admission, DecodeError> {
    let c = context(request)?;
    if !project_x_platform::validate_view(&c, cols, rows) {
        return Err(DecodeError::Shape);
    }
    let serialized = serde_json::to_vec(request).map_err(|_| DecodeError::Shape)?;
    let binding: [u8; 32] = Sha256::digest(&serialized).into();
    Ok(ledger.reserve(&request.request_id, binding, &c))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_view_context_fence_rejects_stale_admission() {
        let original = project_x_protocol::decode(include_bytes!(
            "../../../tests/fixtures/input/request.json"
        ))
        .unwrap();
        for field in 0..8 {
            let mut request = original.clone();
            match field {
                0 => request.context.session = "other".into(),
                1 => request.context.incarnation = "other".into(),
                2 => request.context.endpoint = "other".into(),
                3 => request.context.generation = "2".into(),
                4 => request.context.prefix = "6".into(),
                5 => request.context.geometry = "2".into(),
                6 => request.context.profile = "2".into(),
                _ => request.context.focus = "2".into(),
            }
            let mut ledger = Ledger::new(context(&original).unwrap(), 4);
            assert_eq!(
                reserve_offline(&mut ledger, &request, 80, 24).unwrap(),
                Admission::Stale
            );
        }
    }
    #[test]
    fn stale_focus_and_unknown_are_not_writes() {
        let r = project_x_protocol::decode(include_bytes!(
            "../../../tests/fixtures/input/request.json"
        ))
        .unwrap();
        let mut l = Ledger::new(context(&r).unwrap(), 4);
        let mut wrong = r.clone();
        wrong.context.focus = "2".into();
        assert_eq!(
            reserve_offline(&mut l, &wrong, 80, 24).unwrap(),
            Admission::Stale
        );
        assert_eq!(
            reserve_offline(&mut l, &r, 80, 24).unwrap(),
            Admission::Reserved
        );
        l.mark_transport_lost();
        assert_eq!(
            reserve_offline(&mut l, &r, 80, 24).unwrap(),
            Admission::Existing(project_x_domain::Delivery::Unknown)
        );
    }
}
