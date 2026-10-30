// Bounded synthetic fixture errors only; never echo raw terminal output or env.
export function fixtureFailureSummary(stdout) {
  const line = stdout.trimEnd().split("\n").at(-1);
  if (!line || line.length > 8192) return null;
  let report;
  try {
    report = JSON.parse(line);
  } catch {
    return null;
  }
  if (report?.verdict !== "FAIL" || !report.errors) return null;
  const errors = {};
  for (const key of ["execution_error", "cleanup_error", "seal_error"]) {
    const error = report.errors[key];
    if (typeof error?.type === "string" && typeof error.message === "string")
      errors[key] = {
        type: error.type.slice(0, 128),
        message: error.message.slice(0, 512),
      };
  }
  // JSON escaping keeps terminal controls/newlines inert on the console.
  return Object.keys(errors).length ? JSON.stringify(errors) : null;
}
