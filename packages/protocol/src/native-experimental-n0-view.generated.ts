/* Generated N0 design DTOs; do not edit. No admission or runtime API. */

/**
 * Local product UI plane only. Not an InputIntent or native control operation.
 */
export type ViewCommand =
  | {
      command: "select_view";
      endpoint_id: Id;
      expected: ViewBinding;
    }
  | {
      command: "open_palette";
      endpoint_id: Id;
      expected: ViewBinding;
    };
export type Id = string;
/**
 * Nonzero canonical decimal shape only; exact u64 validation remains unimplemented.
 */
export type Revision = string;

export interface ViewBinding {
  focus_generation: Revision;
  mapping_revision: Revision;
  view_id: Id;
}
