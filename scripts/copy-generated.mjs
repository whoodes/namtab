import { copyFile } from "node:fs/promises";
for (const file of ["validate.generated.mjs", "validate.generated.d.mts"])
  await copyFile(
    "packages/protocol/src/" + file,
    "packages/protocol/dist/" + file,
  );
