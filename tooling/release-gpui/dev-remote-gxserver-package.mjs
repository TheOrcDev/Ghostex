/**
 * CDXC:RemoteMachines 2026-10-01 WHY:
 * A macOS `cargo xtask start` bundled whatever Linux gxserver package it found, and nothing on a Mac rebuilds one, so the dev app kept uploading a package from 2026-08-05 whose `gxserver setup` rejected `--analytics-role` and every Linux Connect failed with "Remote gxserver install failed". When no locally built package matches the checkout, the local start stages the published package of the checkout's release (the latest release when that version is not out yet), verified against GitHub's asset digest and cached per tag, and never the leftover copy.
 * SEE-ALSO: `resolve_local_start_remote_gxserver_linux_package` in apps/desktop/scripts/build-macos-app.sh.
 */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, renameSync, rmSync, statSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const repository = 'maddada/Ghostex';

function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith('--') || !value || value.startsWith('--')) {
      throw new Error('Usage: dev-remote-gxserver-package.mjs --arch x64|arm64 --version VERSION --cache-root DIR');
    }
    options[key.slice(2)] = value;
  }
  for (const required of ['arch', 'version', 'cache-root']) {
    if (!options[required]) throw new Error(`Missing --${required}`);
  }
  if (!['x64', 'arm64'].includes(options.arch)) {
    throw new Error(`Unsupported Linux package arch: ${options.arch}`);
  }
  return options;
}

/** @returns {Promise<any>} */
async function githubJson(pathname) {
  const headers = { Accept: 'application/vnd.github+json' };
  const token = process.env.GH_TOKEN || process.env.GITHUB_TOKEN;
  if (token) headers.Authorization = `Bearer ${token}`;
  const response = await fetch(`https://api.github.com/repos/${repository}/${pathname}`, { headers });
  if (response.status === 404) return undefined;
  if (!response.ok) {
    throw new Error(`GitHub ${pathname}: HTTP ${response.status} ${response.statusText}`);
  }
  return response.json();
}

async function publishedRelease(version) {
  return (
    (await githubJson(`releases/tags/${encodeURIComponent(`v${version}`)}`)) ?? (await githubJson('releases/latest'))
  );
}

const options = parseArgs(process.argv.slice(2));
const assetName = `gxserver-linux-${options.arch}.tar.gz`;
const versionDir = path.join(options['cache-root'], `v${options.version}`, options.arch);
const cachedPackage = path.join(versionDir, 'package');
// A stand-in from the latest release (this version is not published yet) is re-checked twice a day.
const fallbackMarker = path.join(versionDir, 'latest-release-stand-in');
const fallbackExpired =
  existsSync(fallbackMarker) && Date.now() - statSync(fallbackMarker).mtimeMs > 12 * 60 * 60 * 1000;
if (existsSync(path.join(cachedPackage, 'bin', 'gxserver')) && !fallbackExpired) {
  process.stdout.write(`${cachedPackage}\n`);
  process.exit(0);
}

const release = await publishedRelease(options.version);
const asset = release?.assets?.find((candidate) => candidate.name === assetName);
const sha256 = /^sha256:([0-9a-f]{64})$/u.exec(asset?.digest ?? '')?.[1];
if (!asset || !sha256) {
  throw new Error(`No published ${assetName} with a digest for Ghostex ${options.version} or the latest release.`);
}
const response = await fetch(asset.browser_download_url);
if (!response.ok) {
  throw new Error(`${asset.browser_download_url}: HTTP ${response.status} ${response.statusText}`);
}
const archive = Buffer.from(await response.arrayBuffer());
const actual = createHash('sha256').update(archive).digest('hex');
if (actual !== sha256) {
  throw new Error(`${assetName} from ${release.tag_name} failed its checksum (expected ${sha256}, got ${actual}).`);
}

const stageDir = path.join(versionDir, `.package-${process.pid}`);
const archivePath = path.join(versionDir, `.${assetName}-${process.pid}`);
rmSync(stageDir, { recursive: true, force: true });
mkdirSync(stageDir, { recursive: true });
try {
  writeFileSync(archivePath, archive);
  const tar = spawnSync('tar', ['-xzf', archivePath, '-C', stageDir], { stdio: 'inherit' });
  if (tar.status !== 0) throw new Error(`Could not extract ${assetName}.`);
  rmSync(cachedPackage, { recursive: true, force: true });
  renameSync(stageDir, cachedPackage);
  if (release.tag_name === `v${options.version}`) rmSync(fallbackMarker, { force: true });
  else writeFileSync(fallbackMarker, `${release.tag_name}\n`);
} finally {
  rmSync(archivePath, { force: true });
  rmSync(stageDir, { recursive: true, force: true });
}
process.stderr.write(`Using the published ${assetName} from ${release.tag_name}.\n`);
process.stdout.write(`${cachedPackage}\n`);
