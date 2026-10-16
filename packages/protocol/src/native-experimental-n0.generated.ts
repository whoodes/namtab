/* Generated N0 design DTOs; do not edit. No admission or runtime API. */

/**
 * Read correlation is not a mutation identity. Input requests use the issued key above.
 */
export type Message =
  | {
      limits: ReceiptLimits;
      message: "receipt_profile_proposal";
    }
  | {
      correlation_id: Id;
      credit: Credit;
      message: "subscribe";
      session: SessionRef;
      start: SubscriptionStart;
    }
  | {
      correlation_id: Id;
      message: "subscription_opened";
      projection: Projection;
      session: SessionRef;
      subscription_id: Id;
    }
  | {
      correlation_id: Id;
      message: "subscribe_refused";
      reason: SubscribeRefusalReason;
    }
  | {
      cursor: Id;
      message: "read_page";
      subscription_id: Id;
    }
  | {
      credit: Credit;
      message: "grant_credit";
      subscription_id: Id;
    }
  | {
      message: "cancel_subscription";
      subscription_id: Id;
    }
  | {
      message: "snapshot_page";
      page: SnapshotPage;
    }
  | {
      message: "observation_unavailable";
      projection: Projection;
      reason: UnavailableReason;
      session: SessionRef;
      subscription_id: Id;
    }
  | {
      delta: FrameDelta;
      message: "frame_delta";
    }
  | {
      message: "resync_required";
      reason: ResyncReason;
      subscription_id: Id;
    }
  | {
      message: "subscription_closed";
      reason: CloseReason;
      subscription_id: Id;
    }
  | {
      message: "input";
      request: InputRequest;
    };
export type Id = string;
/**
 * Nonzero canonical decimal shape only; exact u64 validation remains unimplemented.
 */
export type Revision = string;
/**
 * Canonical decimal shape only; semantic validation must also enforce <= u64::MAX.
 */
export type Counter = string;
export type SubscriptionStart =
  | {
      kind: "current_snapshot";
      projection: Projection;
    }
  | {
      base: FrameStamp;
      cursor: Id;
      kind: "resume";
    };
export type Projection =
  | {
      kind: "active_grid";
    }
  | {
      buffer: BufferRef;
      first_row_id: Id;
      kind: "history";
      limit: number;
    };
export type BufferKind = "primary" | "alternate";
/**
 * Read-only refusal codes use the existing error families; no mutation decision is recorded.
 */
export type SubscribeRefusalReason =
  | "invalid_request"
  | "unsupported_capability"
  | "unauthorized"
  | "stale_context"
  | "resource_exhausted"
  | "unavailable"
  | "incompatible_version";
export type Completeness = "complete_for_selection" | "partial";
export type UnavailableReason = "denied" | "expired" | "capture_gap" | "state_unknown";
export type FactEvidence =
  | {
      claim: SessionClaim;
      level: "observed";
      producer: Producer;
      schema: Id;
      source: EvidenceRef;
    }
  | {
      channel_id: Id;
      claim: SessionClaim;
      level: "reported";
      producer: Producer;
      schema: Id;
      source: EvidenceRef;
    }
  | {
      claim: SessionClaim;
      /**
       * @minItems 1
       * @maxItems 8
       */
      evidence: EvidenceRef[];
      level: "inferred";
      method: Id;
      producer: Producer;
    }
  | {
      claim: ClaimKind;
      last_known?: Id;
      level: "unavailable";
      reason: UnavailableReason;
    };
export type SessionClaim =
  | {
      kind: "interpretation";
      known: boolean;
    }
  | {
      kind: "activity";
      text: string;
    };
export type EvidenceRef =
  | {
      kind: "session_event";
      sequence: Counter;
      session: SessionRef;
    }
  | {
      kind: "control_record";
      record: RecordRef;
    };
export type ClaimKind = "interpretation" | "activity";
export type CursorShape = "block" | "underline" | "bar";
export type Occupancy = "single" | "wide_lead" | "wide_tail";
export type Color =
  | {
      kind: "default";
    }
  | {
      index: number;
      kind: "indexed";
    }
  | {
      blue: number;
      green: number;
      kind: "rgb";
      red: number;
    };
export type StyleFlag = "bold" | "faint" | "italic" | "underline" | "blink" | "inverse" | "conceal" | "strike";
export type ResyncReason =
  | "missing_base"
  | "presentation_gap"
  | "expired_cursor"
  | "policy_changed"
  | "target_replaced"
  | "credit_exhausted";
export type CloseReason = "canceled" | "disconnected" | "denied" | "retired";
export type ApprovalBinding =
  | {
      kind: "not_required";
      policy_revision: Revision;
    }
  | {
      approval: RecordRef;
      kind: "required";
    };
export type InputRoute =
  | {
      kind: "direct_session";
    }
  | {
      binding: ViewBinding;
      kind: "view";
    };
export type InputIntent =
  | {
      action: KeyAction;
      facts: KeyFacts;
      kind: "send_key";
    }
  | {
      kind: "commit_text";
      text: string;
    }
  | {
      fallback: PasteFallback;
      kind: "paste";
      newlines: NewlinePolicy;
      text: string;
    }
  | {
      gained: boolean;
      kind: "focus_report";
      view: ViewBinding;
    };
export type KeyAction = "press" | "repeat" | "release";
export type Modifier = "shift" | "alt" | "control" | "meta" | "alt_graph";
export type LogicalKey =
  | {
      key: NamedKey;
      kind: "named";
    }
  | {
      kind: "character";
      text: string;
    };
export type NamedKey =
  | "Up"
  | "Down"
  | "Left"
  | "Right"
  | "Tab"
  | "Enter"
  | "Escape"
  | "Backspace"
  | "Home"
  | "End"
  | "PageUp"
  | "PageDown"
  | "Insert"
  | "Delete"
  | "F1"
  | "F2"
  | "F3"
  | "F4"
  | "F5"
  | "F6"
  | "F7"
  | "F8"
  | "F9"
  | "F10"
  | "F11"
  | "F12"
  | "KeypadEnter"
  | "Keypad0"
  | "Keypad1"
  | "Keypad2"
  | "Keypad3"
  | "Keypad4"
  | "Keypad5"
  | "Keypad6"
  | "Keypad7"
  | "Keypad8"
  | "Keypad9"
  | "KeypadAdd"
  | "KeypadSubtract"
  | "KeypadMultiply"
  | "KeypadDivide"
  | "KeypadDecimal";
export type PasteFallback = "reject" | "explicit_unbracketed";
export type NewlinePolicy = "preserve" | "lf_from_crlf_v1";
export type Namespace = "namtab.native.experimental.n0";
export type Version = "n0-06-draft-1";

export interface Envelope {
  body: Message;
  namespace: Namespace;
  version: Version;
}
export interface ReceiptLimits {
  clock_unit: Id;
  max_binding_bytes: number;
  max_sequence: Revision;
  namespace_lifetime_ticks: Counter;
  namespace_records_per_principal: number;
  namespace_records_total: number;
  profile: RecordRef;
  receipt_slots_per_principal: number;
  receipt_slots_total: number;
  retention_after_decision_ticks: Counter;
  volatile_bindings_per_principal: number;
  volatile_bindings_total: number;
}
export interface RecordRef {
  id: Id;
  revision: Revision;
}
export interface Credit {
  bytes: number;
  pages: number;
}
export interface SessionRef {
  incarnation: Revision;
  namespace_id: Id;
  session_id: Id;
}
export interface BufferRef {
  generation: Revision;
  kind: BufferKind;
}
export interface FrameStamp {
  buffer: BufferRef;
  capability_revision: Revision;
  geometry: Geometry;
  mode_revision: Revision;
  observation_id: Id;
  policy: PolicyTuple;
  processed_frontier: Counter;
  semantic_revision: Revision;
  session: SessionRef;
  source_frontier: Counter;
}
export interface Geometry {
  columns: number;
  revision: Revision;
  rows: number;
}
export interface PolicyTuple {
  engine_revision: Id;
  profile: RecordRef;
  reflow_policy: Id;
  text_policy: Id;
  unicode_data?: Id;
}
export interface SnapshotPage {
  coverage: Coverage;
  /**
   * @maxItems 16
   */
  facts: Fact[];
  next_cursor?: Id;
  page_id: Id;
  presentation: Presentation;
  projection: Projection;
  /**
   * @maxItems 32
   */
  rows: PresentationRow[];
  snapshot_id: Id;
  stamp: FrameStamp;
  start_row: Counter;
  subscription_id: Id;
  total_rows: Counter;
}
export interface Coverage {
  capture_policy: RecordRef;
  completeness: Completeness;
  /**
   * @maxItems 8
   */
  gaps: RowGap[];
}
export interface RowGap {
  end: Counter;
  reason: UnavailableReason;
  start: Counter;
}
export interface Fact {
  evidence: FactEvidence;
  subject: SessionRef;
}
export interface Producer {
  component_id: Id;
  incarnation: Revision;
}
export interface Presentation {
  cursor?: Cursor;
}
export interface Cursor {
  blink: boolean;
  column: number;
  row: number;
  shape: CursorShape;
  visible: boolean;
}
export interface PresentationRow {
  /**
   * @minItems 2
   * @maxItems 500
   */
  cells: Cell[];
  revision: Revision;
  row_id: Id;
  soft_wrapped: boolean;
}
export interface Cell {
  occupancy: Occupancy;
  style: Style;
  text: string;
}
export interface Style {
  background: Color;
  /**
   * @maxItems 8
   */
  flags: StyleFlag[];
  foreground: Color;
}
export interface FrameDelta {
  base: FrameStamp;
  /**
   * @maxItems 16
   */
  facts: Fact[];
  presentation: Presentation;
  /**
   * @maxItems 32
   */
  replace_rows: RowReplacement[];
  resume_cursor: Id;
  subscription_id: Id;
  target: FrameStamp;
}
export interface RowReplacement {
  index: number;
  row: PresentationRow;
}
export interface InputRequest {
  context: InputContext;
  intent: InputIntent;
  key: MutationKey;
}
export interface InputContext {
  approval: ApprovalBinding;
  context: RecordRef;
  deadline: Deadline;
  endpoint_capability_revision: Revision;
  endpoint_generation: Revision;
  endpoint_id: Id;
  expected: FrameStamp;
  grant: RecordRef;
  helper_epoch: Revision;
  protection_revision: Revision;
  receipt_profile: RecordRef;
  route: InputRoute;
  writer_generation: Revision;
}
export interface Deadline {
  clock_origin_id: Id;
  ticks: Counter;
}
export interface ViewBinding {
  focus_generation: Revision;
  mapping_revision: Revision;
  view_id: Id;
}
export interface KeyFacts {
  composing?: boolean;
  /**
   * @maxItems 5
   */
  consumed_modifiers?: Modifier[];
  layout_id?: Id;
  logical?: LogicalKey;
  /**
   * @maxItems 5
   */
  modifiers?: Modifier[];
  phase?: KeyAction;
  physical_code?: Id;
}
export interface MutationKey {
  authority_epoch: Revision;
  namespace_id: Id;
  request_namespace_id: Id;
  request_seq: Revision;
}
