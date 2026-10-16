import { checkNativeFixtures } from "./check-native-fixtures.mjs";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
const manifest = JSON.parse(
  await readFile("tests/fixtures/manifest.json", "utf8"),
);
for (const item of manifest.files) {
  const data = await readFile("tests/fixtures/" + item.path);
  if (createHash("sha256").update(data).digest("hex") !== item.sha256)
    throw new Error("Fixture drift: " + item.path);
}
console.log(
  "Sanitized fixture manifest verified (" +
    manifest.files.length +
    " payloads).",
);
console.log("Native fixture static checks:", await checkNativeFixtures());
