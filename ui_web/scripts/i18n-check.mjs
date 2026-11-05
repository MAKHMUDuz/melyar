import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join } from "node:path";

const root = new URL("../src/lib/i18n/locales/", import.meta.url).pathname;
const en = JSON.parse(readFileSync(join(root, "en", "translation.json"), "utf8"));
const enKeys = new Set(Object.keys(en));

const dirs = readdirSync(root, { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .map((d) => d.name)
  .sort();

let totalMissing = 0;
let totalExtra = 0;
for (const code of dirs) {
  const file = join(root, code, "translation.json");
  if (!existsSync(file)) continue;
  const data = JSON.parse(readFileSync(file, "utf8"));
  const keys = new Set(Object.keys(data));
  const missing = [...enKeys].filter((k) => !keys.has(k));
  const extra = [...keys].filter((k) => !enKeys.has(k));
  totalMissing += missing.length;
  totalExtra += extra.length;
  const status = missing.length === 0 && extra.length === 0 ? "OK" : "DRIFT";
  console.log(`${status.padEnd(6)} ${code.padEnd(10)} keys=${keys.size} missing=${missing.length} extra=${extra.length}`);
  if (missing.length) {
    for (const k of missing.slice(0, 8)) console.log(`         - ${k}`);
    if (missing.length > 8) console.log(`         ... and ${missing.length - 8} more`);
  }
}
console.log(`\nTotal missing: ${totalMissing}, extra: ${totalExtra}`);
if (totalMissing > 0) process.exitCode = 1;
