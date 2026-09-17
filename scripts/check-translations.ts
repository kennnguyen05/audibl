// Verifies every locale has exactly the same keys as English.
// Run: bun run check:translations
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const LOCALES_DIR = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "src",
  "i18n",
  "locales",
);
const REFERENCE = "en";

type Tree = { [key: string]: string | Tree };

function keyPaths(tree: Tree, prefix = ""): string[] {
  return Object.entries(tree).flatMap(([key, value]) => {
    const full = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "string") return value.trim() ? [full] : [`${full} (empty)`];
    return keyPaths(value, full);
  });
}

function load(lang: string): Tree {
  return JSON.parse(
    fs.readFileSync(path.join(LOCALES_DIR, lang, "translation.json"), "utf8"),
  );
}

const reference = new Set(keyPaths(load(REFERENCE)));
const languages = fs
  .readdirSync(LOCALES_DIR, { withFileTypes: true })
  .filter((e) => e.isDirectory() && e.name !== REFERENCE)
  .map((e) => e.name);

let failed = [...reference].some((k) => k.endsWith("(empty)"));
for (const lang of languages) {
  const keys = new Set(keyPaths(load(lang)));
  const missing = [...reference].filter((k) => !keys.has(k));
  const extra = [...keys].filter((k) => !reference.has(k));
  if (missing.length || extra.length) {
    failed = true;
    console.error(`✗ ${lang}`);
    missing.forEach((k) => console.error(`  missing: ${k}`));
    extra.forEach((k) => console.error(`  extra:   ${k}`));
  } else {
    console.log(`✓ ${lang}: ${keys.size} keys match ${REFERENCE}`);
  }
}

process.exit(failed ? 1 : 0);
