import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';

const args = Object.fromEntries(process.argv.slice(2).map((value, index, all) => value.startsWith('--') ? [value.slice(2), all[index + 1]] : null).filter(Boolean));
const version = String(args.version || '').replace(/^v/, '');
const repository = args.repository || process.env.GITHUB_REPOSITORY;
const root = resolve(args.directory || 'release');
const output = resolve(args.output || join(root, 'latest.json'));
if (!version || !repository) throw new Error('需要 --version 和 --repository');

function files(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? files(path) : [path];
  });
}

const all = files(root);
const pick = (pattern) => all.find((path) => pattern.test(basename(path)));
const windows = pick(/\.exe$/i);
const windowsSignature = windows && all.find((path) => basename(path) === `${basename(windows)}.sig`);
const mac = pick(/\.app\.tar\.gz$/i);
const macSignature = mac && all.find((path) => basename(path) === `${basename(mac)}.sig`);
const linux = pick(/\.AppImage$/);
const linuxSignature = linux && all.find((path) => basename(path) === `${basename(linux)}.sig`);
for (const [name, path] of Object.entries({ windows, windowsSignature, mac, macSignature, linux, linuxSignature })) {
  if (!path) throw new Error(`缺少 updater 产物：${name}`);
}

const base = `https://github.com/${repository}/releases/download/v${version}/`;
const platform = (path, signaturePath) => ({
  signature: readFileSync(signaturePath, 'utf8').trim(),
  url: base + encodeURIComponent(basename(path)),
});
const manifest = {
  version,
  notes: `OJ Insight v${version}`,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': platform(windows, windowsSignature),
    'linux-x86_64': platform(linux, linuxSignature),
    'darwin-x86_64': platform(mac, macSignature),
    'darwin-aarch64': platform(mac, macSignature),
  },
};
writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Generated ${output}`);
