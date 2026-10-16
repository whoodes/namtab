//! Experimental N0-06 public projection/input DTOs, separate from public-v1 and EA1.
//! Bounds are a proposed static review profile, not accepted product admission limits.
//! Serde/schema shape checks do not validate authority, currentness, UTF-8 byte budgets,
//! exact u64 range, page assembly, row occupancy or any cross-field semantic invariant.
//! Deliberately no Debug derives: input, cell text and claims can contain secrets.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// An omitted fact is unknown. A present null is not another spelling of absence.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Namespace {
    #[serde(rename = "namtab.native.experimental.n0")]
    NativeN0,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Version {
    #[serde(rename = "n0-06-draft-1")]
    Draft1,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Id(
    #[schemars(length(min = 1, max = 128), pattern(r"^[A-Za-z0-9][A-Za-z0-9._:-]*$"))] pub String,
);
/// Canonical decimal shape only; semantic validation must also enforce <= u64::MAX.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Counter(
    #[schemars(length(min = 1, max = 20), pattern(r"^(0|[1-9][0-9]*)$"))] pub String,
);
/// Nonzero canonical decimal shape only; exact u64 validation remains unimplemented.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Revision(#[schemars(length(min = 1, max = 20), pattern(r"^[1-9][0-9]*$"))] pub String);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionRef {
    pub namespace_id: Id,
    pub session_id: Id,
    pub incarnation: Revision,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordRef {
    pub id: Id,
    pub revision: Revision,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Geometry {
    pub revision: Revision,
    #[schemars(range(min = 2, max = 200))]
    pub rows: u16,
    #[schemars(range(min = 2, max = 500))]
    pub columns: u16,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BufferKind {
    Primary,
    Alternate,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BufferRef {
    pub kind: BufferKind,
    pub generation: Revision,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyTuple {
    pub engine_revision: Id,
    pub profile: RecordRef,
    pub text_policy: Id,
    pub reflow_policy: Id,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[schemars(with = "Id")]
    pub unicode_data: Option<Id>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrameStamp {
    pub session: SessionRef,
    pub observation_id: Id,
    pub semantic_revision: Revision,
    pub processed_frontier: Counter,
    pub source_frontier: Counter,
    pub geometry: Geometry,
    pub buffer: BufferRef,
    pub policy: PolicyTuple,
    pub mode_revision: Revision,
    pub capability_revision: Revision,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Color {
    Default {},
    Indexed { index: u8 },
    Rgb { red: u8, green: u8, blue: u8 },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StyleFlag {
    Bold,
    Faint,
    Italic,
    Underline,
    Blink,
    Inverse,
    Conceal,
    Strike,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub foreground: Color,
    pub background: Color,
    #[schemars(length(max = 8))]
    pub flags: Vec<StyleFlag>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Occupancy {
    Single,
    WideLead,
    WideTail,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    #[schemars(length(max = 16))]
    pub text: String,
    pub occupancy: Occupancy,
    pub style: Style,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PresentationRow {
    pub row_id: Id,
    pub revision: Revision,
    pub soft_wrapped: bool,
    #[schemars(length(min = 2, max = 500))]
    pub cells: Vec<Cell>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CursorShape {
    Block,
    Underline,
    Bar,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    pub row: u16,
    pub column: u16,
    pub visible: bool,
    pub blink: bool,
    pub shape: CursorShape,
}
// Presentation metadata is not parser state, a mode inventory or a recovery image.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Presentation {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[schemars(with = "Cursor")]
    pub cursor: Option<Cursor>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    pub component_id: Id,
    pub incarnation: Revision,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvidenceRef {
    SessionEvent {
        session: SessionRef,
        sequence: Counter,
    },
    ControlRecord {
        record: RecordRef,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionClaim {
    Interpretation {
        known: bool,
    },
    Activity {
        #[schemars(length(max = 256))]
        text: String,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    Interpretation,
    Activity,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    Denied,
    Expired,
    CaptureGap,
    StateUnknown,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "level", rename_all = "snake_case", deny_unknown_fields)]
pub enum FactEvidence {
    Observed {
        claim: SessionClaim,
        producer: Producer,
        schema: Id,
        source: EvidenceRef,
    },
    Reported {
        claim: SessionClaim,
        producer: Producer,
        channel_id: Id,
        schema: Id,
        source: EvidenceRef,
    },
    Inferred {
        claim: SessionClaim,
        producer: Producer,
        method: Id,
        #[schemars(length(min = 1, max = 8))]
        evidence: Vec<EvidenceRef>,
    },
    Unavailable {
        claim: ClaimKind,
        reason: UnavailableReason,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present"
        )]
        #[schemars(with = "Id")]
        last_known: Option<Id>,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    pub subject: SessionRef,
    pub evidence: FactEvidence,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Credit {
    #[schemars(range(max = 8))]
    pub pages: u8,
    #[schemars(range(max = 1_048_576))]
    pub bytes: u32,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Projection {
    ActiveGrid {},
    History {
        buffer: BufferRef,
        first_row_id: Id,
        #[schemars(range(min = 1, max = 1000))]
        limit: u16,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SubscriptionStart {
    CurrentSnapshot { projection: Projection },
    Resume { cursor: Id, base: Box<FrameStamp> },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RowGap {
    pub start: Counter,
    pub end: Counter,
    pub reason: UnavailableReason,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    CompleteForSelection,
    Partial,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub completeness: Completeness,
    pub capture_policy: RecordRef,
    #[schemars(length(max = 8))]
    pub gaps: Vec<RowGap>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SnapshotPage {
    pub subscription_id: Id,
    pub snapshot_id: Id,
    pub stamp: FrameStamp,
    pub page_id: Id,
    pub projection: Projection,
    pub start_row: Counter,
    pub total_rows: Counter,
    pub coverage: Coverage,
    #[schemars(length(max = 32))]
    pub rows: Vec<PresentationRow>,
    pub presentation: Presentation,
    #[schemars(length(max = 16))]
    pub facts: Vec<Fact>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[schemars(with = "Id")]
    pub next_cursor: Option<Id>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RowReplacement {
    pub index: u16,
    pub row: PresentationRow,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrameDelta {
    pub subscription_id: Id,
    pub base: FrameStamp,
    pub target: FrameStamp,
    #[schemars(length(max = 32))]
    pub replace_rows: Vec<RowReplacement>,
    pub presentation: Presentation,
    #[schemars(length(max = 16))]
    pub facts: Vec<Fact>,
    pub resume_cursor: Id,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResyncReason {
    MissingBase,
    PresentationGap,
    ExpiredCursor,
    PolicyChanged,
    TargetReplaced,
    CreditExhausted,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    Canceled,
    Disconnected,
    Denied,
    Retired,
}

// Carries the existing finite N0-02 review model; never an accepted product limit.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReceiptLimits {
    pub profile: RecordRef,
    pub max_sequence: Revision,
    pub namespace_lifetime_ticks: Counter,
    pub retention_after_decision_ticks: Counter,
    pub namespace_records_per_principal: u16,
    pub namespace_records_total: u16,
    pub receipt_slots_per_principal: u16,
    pub receipt_slots_total: u16,
    pub volatile_bindings_per_principal: u16,
    pub volatile_bindings_total: u16,
    #[schemars(range(min = 1, max = 4_294_967_295_u32))]
    pub max_binding_bytes: u32,
    pub clock_unit: Id,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MutationKey {
    pub namespace_id: Id,
    pub request_namespace_id: Id,
    pub authority_epoch: Revision,
    pub request_seq: Revision,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Deadline {
    pub clock_origin_id: Id,
    pub ticks: Counter,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ViewBinding {
    pub view_id: Id,
    pub mapping_revision: Revision,
    pub focus_generation: Revision,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputRoute {
    DirectSession {},
    View { binding: ViewBinding },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ApprovalBinding {
    NotRequired { policy_revision: Revision },
    Required { approval: RecordRef },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InputContext {
    pub receipt_profile: RecordRef,
    pub endpoint_id: Id,
    pub endpoint_generation: Revision,
    pub endpoint_capability_revision: Revision,
    pub helper_epoch: Revision,
    pub writer_generation: Revision,
    pub expected: FrameStamp,
    pub route: InputRoute,
    pub protection_revision: Revision,
    pub grant: RecordRef,
    pub context: RecordRef,
    pub approval: ApprovalBinding,
    pub deadline: Deadline,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Shift,
    Alt,
    Control,
    Meta,
    AltGraph,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    Press,
    Repeat,
    Release,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum NamedKey {
    Up,
    Down,
    Left,
    Right,
    Tab,
    Enter,
    Escape,
    Backspace,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    KeypadEnter,
    Keypad0,
    Keypad1,
    Keypad2,
    Keypad3,
    Keypad4,
    Keypad5,
    Keypad6,
    Keypad7,
    Keypad8,
    Keypad9,
    KeypadAdd,
    KeypadSubtract,
    KeypadMultiply,
    KeypadDivide,
    KeypadDecimal,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LogicalKey {
    Named {
        key: NamedKey,
    },
    Character {
        #[schemars(length(min = 1, max = 1))]
        text: String,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KeyFacts {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[schemars(with = "LogicalKey")]
    pub logical: Option<LogicalKey>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[schemars(length(max = 5))]
    #[schemars(with = "Vec<Modifier>")]
    pub modifiers: Option<Vec<Modifier>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[schemars(with = "Id")]
    pub physical_code: Option<Id>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[schemars(with = "Id")]
    pub layout_id: Option<Id>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[schemars(length(max = 5))]
    #[schemars(with = "Vec<Modifier>")]
    pub consumed_modifiers: Option<Vec<Modifier>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[schemars(with = "bool")]
    pub composing: Option<bool>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    #[schemars(with = "KeyAction")]
    pub phase: Option<KeyAction>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NewlinePolicy {
    Preserve,
    LfFromCrlfV1,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PasteFallback {
    Reject,
    ExplicitUnbracketed,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputIntent {
    SendKey {
        action: KeyAction,
        facts: KeyFacts,
    },
    CommitText {
        #[schemars(length(max = 8192))]
        text: String,
    },
    Paste {
        #[schemars(length(max = 8192))]
        text: String,
        newlines: NewlinePolicy,
        fallback: PasteFallback,
    },
    FocusReport {
        view: ViewBinding,
        gained: bool,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InputRequest {
    pub key: MutationKey,
    pub context: InputContext,
    pub intent: InputIntent,
}
/// Read-only refusal codes use the existing error families; no mutation decision is recorded.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubscribeRefusalReason {
    InvalidRequest,
    UnsupportedCapability,
    Unauthorized,
    StaleContext,
    ResourceExhausted,
    Unavailable,
    IncompatibleVersion,
}
/// Read correlation is not a mutation identity. Input requests use the issued key above.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "message", rename_all = "snake_case", deny_unknown_fields)]
pub enum Message {
    ReceiptProfileProposal {
        limits: ReceiptLimits,
    },
    Subscribe {
        correlation_id: Id,
        session: SessionRef,
        start: SubscriptionStart,
        credit: Credit,
    },
    SubscriptionOpened {
        correlation_id: Id,
        subscription_id: Id,
        session: SessionRef,
        projection: Projection,
    },
    SubscribeRefused {
        correlation_id: Id,
        reason: SubscribeRefusalReason,
    },
    ReadPage {
        subscription_id: Id,
        cursor: Id,
    },
    GrantCredit {
        subscription_id: Id,
        credit: Credit,
    },
    CancelSubscription {
        subscription_id: Id,
    },
    SnapshotPage {
        page: SnapshotPage,
    },
    ObservationUnavailable {
        subscription_id: Id,
        session: SessionRef,
        projection: Projection,
        reason: UnavailableReason,
    },
    FrameDelta {
        delta: FrameDelta,
    },
    ResyncRequired {
        subscription_id: Id,
        reason: ResyncReason,
    },
    SubscriptionClosed {
        subscription_id: Id,
        reason: CloseReason,
    },
    Input {
        request: InputRequest,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub namespace: Namespace,
    pub version: Version,
    pub body: Message,
}

/// Local product UI plane only. Not an InputIntent or native control operation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum ViewCommand {
    SelectView {
        endpoint_id: Id,
        expected: ViewBinding,
    },
    OpenPalette {
        endpoint_id: Id,
        expected: ViewBinding,
    },
}

pub fn schema() -> schemars::Schema {
    schemars::generate::SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<Envelope>()
}
pub fn view_schema() -> schemars::Schema {
    schemars::generate::SchemaSettings::draft07()
        .into_generator()
        .into_root_schema_for::<ViewCommand>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn examples() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/native-public/examples.json"
        ))
        .unwrap()
    }

    #[test]
    fn public_examples_roundtrip_through_owned_dtos() {
        let fixture = examples();
        for value in fixture["examples"].as_object().unwrap().values() {
            let envelope: Envelope = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(envelope).unwrap(), *value);
        }
        for value in fixture["view_commands"].as_array().unwrap() {
            let command: ViewCommand = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(command).unwrap(), *value);
        }
    }

    #[test]
    fn absence_false_null_and_duplicate_fields_are_distinct() {
        let mut value = examples()["examples"]["input-key"].clone();
        let envelope: Envelope = serde_json::from_value(value.clone()).unwrap();
        let encoded = serde_json::to_value(envelope).unwrap();
        assert!(
            encoded["body"]["request"]["intent"]["facts"]
                .get("composing")
                .is_none()
        );
        value["body"]["request"]["intent"]["facts"]["composing"] = json!(false);
        let envelope: Envelope = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(envelope).unwrap(), value);
        value["body"]["request"]["intent"]["facts"]["composing"] = Value::Null;
        assert!(serde_json::from_value::<Envelope>(value).is_err());
        let encoded = serde_json::to_string(&examples()["examples"]["input-key"]).unwrap();
        let duplicate = encoded.replacen(
            "\"action\":\"press\"",
            "\"action\":\"press\",\"action\":\"press\"",
            1,
        );
        assert_ne!(duplicate, encoded);
        assert!(serde_json::from_str::<Envelope>(&duplicate).is_err());
    }

    #[test]
    fn empty_tagged_variants_reject_raw_unknown_and_duplicate_unknown_fields() {
        // Positive first; the empty-struct Rust forms preserve the original wire JSON.
        let fixture = examples();
        let mut direct = fixture["examples"]["input-key"].clone();
        direct["body"]["request"]["context"]["route"] = json!({"kind":"direct_session"});
        for (value, tag) in [
            (fixture["examples"]["subscribe"].clone(), "active_grid"),
            (fixture["examples"]["snapshot-page-one"].clone(), "default"),
            (direct, "direct_session"),
        ] {
            let original = serde_json::to_string(&value).unwrap();
            let decoded: Envelope = serde_json::from_str(&original).unwrap();
            assert_eq!(serde_json::to_value(decoded).unwrap(), value);
            let needle = format!("\"kind\":\"{tag}\"");
            for extra in [
                "\"unexpected_probe\":true",
                "\"unexpected_probe\":true,\"unexpected_probe\":false",
                "\"binding\":{\"view_id\":\"view-one\",\"mapping_revision\":\"1\",\"focus_generation\":\"2\"}",
            ] {
                // Do not parse the malformed raw string into a Value first: that erases duplicates.
                let raw = original.replacen(&needle, &format!("{needle},{extra}"), 1);
                assert_ne!(raw, original);
                assert!(
                    serde_json::from_str::<Envelope>(&raw).is_err(),
                    "{tag} silently accepted extra fields"
                );
            }
        }
    }

    #[test]
    fn opening_refusal_and_active_unavailability_require_their_routing_fields() {
        let fixture = examples();
        for name in [
            "opened",
            "opened-zero-credit",
            "subscribe-refused",
            "unavailable",
        ] {
            let value = &fixture["examples"][name];
            let raw = serde_json::to_string(value).unwrap();
            let decoded: Envelope = serde_json::from_str(&raw).unwrap();
            assert_eq!(serde_json::to_value(decoded).unwrap(), *value);
            let required: &[&str] = match name {
                "subscribe-refused" => &["correlation_id"],
                "unavailable" => &["subscription_id"],
                _ => &["correlation_id", "subscription_id", "session", "projection"],
            };
            for field in required {
                let mut missing = value.clone();
                assert!(
                    missing["body"]
                        .as_object_mut()
                        .unwrap()
                        .remove(*field)
                        .is_some()
                );
                assert!(
                    serde_json::from_str::<Envelope>(&serde_json::to_string(&missing).unwrap())
                        .is_err()
                );
            }
        }
        let mut refused = fixture["examples"]["subscribe-refused"].clone();
        refused["body"]["subscription_id"] = json!("unallocated-subscription");
        assert!(
            serde_json::from_str::<Envelope>(&serde_json::to_string(&refused).unwrap()).is_err()
        );
    }

    #[test]
    fn public_namespace_and_action_planes_do_not_accept_old_or_private_shapes() {
        let mut value = examples()["examples"]["input-key"].clone();
        value["namespace"] = json!("project-x.public.v1");
        assert!(serde_json::from_value::<Envelope>(value).is_err());
        let mut value = examples()["examples"]["input-key"].clone();
        value["body"]["request"]["intent"] =
            json!({"op":"encode","input":{"kind":"key","key":"Tab"}});
        assert!(serde_json::from_value::<Envelope>(value).is_err());
        let mut value = examples()["examples"]["input-key"].clone();
        value["body"]["request"]["intent"] = examples()["view_commands"][0].clone();
        assert!(serde_json::from_value::<Envelope>(value).is_err());
    }
}
