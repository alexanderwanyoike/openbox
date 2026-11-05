import { cp, mkdir } from "node:fs/promises";
async function build() {
  try {
    await mkdir("site-dist", { recursive: true });
    for (const path of ["index.html", "styles.css", "releases.mjs", "assets"])
      await cp(`site/${path}`, `site-dist/${path}`, { recursive: true });
    return { ok: true };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : "Site build failed",
    };
  }
}
const result = await build();
if (!result.ok) {
  process.stderr.write(`${result.error}\n`);
  process.exitCode = 1;
}
