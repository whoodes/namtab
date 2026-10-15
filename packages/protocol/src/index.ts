import validate from "./validate.generated.mjs";
import type { InputRequest } from "./generated.js";
export type { InputRequest } from "./generated.js";
export function isDecimalU64(s: string): boolean {
  return (
    /^(0|[1-9][0-9]{0,19})$/u.test(s) && BigInt(s) <= 18446744073709551615n
  );
}
// Parsed-value validation only; no duplicate-key/frame decoder in TS yet.
export function validateParsedRequest(value: unknown): value is InputRequest {
  if (!validate(value) || value.version !== 1) return false;
  if (value.intent.kind !== "key" && /[\uD800-\uDFFF]/u.test(value.intent.text))
    return false;
  const c = value.context;
  if (
    ![value.request_id, c.session, c.incarnation, c.endpoint].every((s) =>
      /^[A-Za-z0-9_:-]{1,128}$/u.test(s),
    )
  )
    return false;
  return [c.generation, c.prefix, c.geometry, c.profile, c.focus].every(
    isDecimalU64,
  );
}
