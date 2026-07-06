import { statSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const distAssetsDir = join(scriptDir, '..', '..', 'dist', 'assets');

const BUDGETS = {
  mainEntryMaxKb: 450,
  monacoVendorMaxKb: 3900,
  opencvVendorMaxKb: 50,
};

function toKb(bytes) {
  return bytes / 1024;
}

function findAsset(prefix) {
  const entries = readdirSync(distAssetsDir);
  const match = entries.find((name) => name.startsWith(prefix) && name.endsWith('.js'));
  return match ? join(distAssetsDir, match) : null;
}

function readSize(path) {
  return toKb(statSync(path).size);
}

function fail(message) {
  console.error(`BUNDLE_BUDGET_FAIL: ${message}`);
  process.exitCode = 1;
}

const mainPath = findAsset('main-');
const monacoPath = findAsset('vendor-monaco-');
const opencvPath = findAsset('vendor-opencv-');

if (!mainPath || !monacoPath) {
  fail('Missing required build assets (main or vendor-monaco). Run npm run build first.');
  process.exit(process.exitCode || 1);
}

const mainKb = readSize(mainPath);
const monacoKb = readSize(monacoPath);
const opencvKb = opencvPath ? readSize(opencvPath) : 0;

console.log(`Bundle sizes (KB): main=${mainKb.toFixed(2)}, monaco=${monacoKb.toFixed(2)}, opencv=${opencvKb.toFixed(2)}`);

if (mainKb > BUDGETS.mainEntryMaxKb) {
  fail(`main entry ${mainKb.toFixed(2)}KB exceeds ${BUDGETS.mainEntryMaxKb}KB`);
}

if (monacoKb > BUDGETS.monacoVendorMaxKb) {
  fail(`vendor-monaco ${monacoKb.toFixed(2)}KB exceeds ${BUDGETS.monacoVendorMaxKb}KB`);
}

if (opencvPath && opencvKb > BUDGETS.opencvVendorMaxKb) {
  fail(`vendor-opencv ${opencvKb.toFixed(2)}KB exceeds ${BUDGETS.opencvVendorMaxKb}KB`);
}

if (!process.exitCode) {
  console.log('Bundle budgets: PASS');
}
