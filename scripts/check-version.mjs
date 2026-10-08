import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const packageVersion = JSON.parse(readFileSync(resolve(root, 'package.json'), 'utf8')).version;
const cargo = readFileSync(resolve(root, 'src-tauri', 'Cargo.toml'), 'utf8');
const cargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const tauriVersion = JSON.parse(readFileSync(resolve(root, 'src-tauri', 'tauri.conf.json'), 'utf8')).version;
const frontend = readFileSync(resolve(root, 'src', 'lib', 'version.ts'), 'utf8');
const frontendVersion = frontend.match(/APP_VERSION\s*=\s*['"]([^'"]+)['"]/)?.[1];

const versions = { packageVersion, cargoVersion, tauriVersion, frontendVersion };
const mismatched = Object.entries(versions).filter(([, version]) => version !== packageVersion);
if (mismatched.length) {
  console.error('Version mismatch:', versions);
  process.exit(1);
}
console.log(`Version ${packageVersion} is consistent.`);
