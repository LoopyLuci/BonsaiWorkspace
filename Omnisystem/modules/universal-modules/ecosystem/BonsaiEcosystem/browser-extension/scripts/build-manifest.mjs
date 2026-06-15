import fs from 'node:fs';
import path from 'node:path';

const target = process.argv[2] ?? 'chrome';
const root = process.cwd();
const distDir = path.join(root, 'dist');

const basePath = path.join(root, 'manifest.base.json');
const overridePath = path.join(root, `manifest.${target}.json`);

if (!fs.existsSync(basePath)) {
  throw new Error(`Missing base manifest: ${basePath}`);
}
if (!fs.existsSync(overridePath)) {
  throw new Error(`Missing override manifest: ${overridePath}`);
}
if (!fs.existsSync(distDir)) {
  throw new Error(`Missing dist directory: ${distDir}`);
}

const base = JSON.parse(fs.readFileSync(basePath, 'utf-8'));
const override = JSON.parse(fs.readFileSync(overridePath, 'utf-8'));

const merged = {
  ...base,
  ...override,
  permissions: Array.from(new Set([...(base.permissions ?? []), ...(override.permissions ?? [])])),
  optional_permissions: Array.from(new Set([...(base.optional_permissions ?? []), ...(override.optional_permissions ?? [])]))
};

const outputPath = path.join(distDir, 'manifest.json');
fs.writeFileSync(outputPath, `${JSON.stringify(merged, null, 2)}\n`, 'utf-8');
console.log(`Wrote ${outputPath} for target=${target}`);
