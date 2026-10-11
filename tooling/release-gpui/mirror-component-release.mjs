#!/usr/bin/env node
/*
 * Mirrors one immutable component release (a `cef-<version>` or
 * `code-server-<identity>` tag) from one GitHub repository to another without
 * rebuilding anything: every asset is downloaded, checked against GitHub's own
 * digest metadata and against its filename-bound `.sha256` sidecar, and
 * re-uploaded byte-identical under the same tag in the destination. The
 * destination release is created with the source's title and body when it is
 * missing and is never marked as the repository's latest release.
 *
 * Idempotent: assets that already exist in the destination with the same size
 * and digest are left alone; an asset that exists with different bytes aborts
 * the run, because a component tag may only ever mean one payload.
 *
 * Usage:
 *   node tooling/release-gpui/mirror-component-release.mjs --tag <tag> \
 *     [--from maddada/Ghostex] [--to maddada/ghostex-components] [--dry-run]
 *
 * Writes to the destination authenticate with COMPONENTS_GITHUB_TOKEN when it
 * is set (see components-repo-setup.md); otherwise the ambient `gh` login is
 * used. Reads from the source never need more than public access.
 */
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { LEGACY_COMPONENTS_GITHUB_REPO, componentsGithubEnv, componentsGithubRepo } from './components-repo.mjs';
import { authenticateComponentChecksumSidecar } from './on-demand-manifest.mjs';
import { normalizedRemoteDigest } from './publish-component.mjs';

const repoPattern = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/u;
const componentTagPattern = /^(cef|code-server)-[A-Za-z0-9][A-Za-z0-9._-]*$/u;

function parseArguments(argv) {
  const options = { dryRun: false, from: LEGACY_COMPONENTS_GITHUB_REPO, to: componentsGithubRepo(), tag: null };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === '--dry-run') {
      options.dryRun = true;
      continue;
    }
    if (!['--tag', '--from', '--to'].includes(argument)) throw new Error(`Unexpected argument: ${argument}`);
    const value = argv[index + 1];
    if (!value || value.startsWith('--')) throw new Error(`Missing value for ${argument}`);
    options[argument.slice(2)] = value;
    index += 1;
  }
  if (!options.tag || !componentTagPattern.test(options.tag)) {
    throw new Error('--tag must name a cef-<version> or code-server-<identity> component tag');
  }
  for (const key of ['from', 'to']) {
    if (!repoPattern.test(options[key])) throw new Error(`--${key} must have owner/repository form`);
  }
  if (options.from === options.to) throw new Error('--from and --to must differ');
  return options;
}

function gh(args, { allowFailure = false, write = false } = {}) {
  const result = spawnSync('gh', args, { encoding: 'utf8', env: write ? componentsGithubEnv() : { ...process.env } });
  if (result.status !== 0 && !allowFailure) {
    throw new Error(`gh ${args.join(' ')} failed: ${(result.stderr || result.stdout || 'unknown error').trim()}`);
  }
  return result;
}

function viewRelease({ repo, tag, write = false }) {
  const result = gh(
    ['release', 'view', tag, '--repo', repo, '--json', 'name,body,isDraft,isPrerelease,tagName,assets,url'],
    { allowFailure: true, write }
  );
  if (result.status !== 0) {
    const diagnostic = `${result.stderr ?? ''}\n${result.stdout ?? ''}`.toLowerCase();
    if (diagnostic.includes('release not found') || diagnostic.includes('not found') || diagnostic.includes('404')) {
      return { exists: false, assets: [] };
    }
    throw new Error(`Could not inspect ${repo} release ${tag}: ${diagnostic.trim() || `gh exited ${result.status}`}`);
  }
  return { exists: true, ...JSON.parse(result.stdout) };
}

function sha256File(filePath) {
  return createHash('sha256').update(readFileSync(filePath)).digest('hex');
}

function sameBytes(remote, local) {
  const remoteDigest = normalizedRemoteDigest(remote);
  return Number(remote.size) === local.sizeBytes && (!remoteDigest || remoteDigest === local.sha256);
}

function planMirror({ source, destination }) {
  const destinationByName = new Map((destination.assets ?? []).map((asset) => [asset.name, asset]));
  const uploads = [];
  const noops = [];
  for (const asset of source.assets) {
    const existing = destinationByName.get(asset.name);
    if (!existing) {
      uploads.push(asset);
      continue;
    }
    const sourceDigest = normalizedRemoteDigest(asset);
    const existingDigest = normalizedRemoteDigest(existing);
    if (
      Number(existing.size) !== Number(asset.size) ||
      (sourceDigest && existingDigest && sourceDigest !== existingDigest)
    ) {
      throw new Error(
        `Refusing to mirror ${asset.name}: destination already has size/digest ${existing.size}/${existingDigest || 'unavailable'}, ` +
          `source is ${asset.size}/${sourceDigest || 'unavailable'}`
      );
    }
    noops.push(asset);
  }
  return { createRelease: !destination.exists, uploads, noops };
}

function verifyDownloadedAssets({ assetDir, assets }) {
  const verified = [];
  const localNames = new Set(readdirSync(assetDir));
  for (const asset of assets) {
    const filePath = path.join(assetDir, asset.name);
    if (!localNames.has(asset.name)) throw new Error(`Download did not produce ${asset.name}`);
    const local = { filePath, name: asset.name, sha256: sha256File(filePath), sizeBytes: statSync(filePath).size };
    if (!sameBytes(asset, local)) {
      throw new Error(
        `Downloaded ${asset.name} does not match GitHub metadata: ${local.sizeBytes}/${local.sha256}; ` +
          `expected ${asset.size}/${normalizedRemoteDigest(asset) || 'unavailable'}`
      );
    }
    verified.push(local);
  }
  const byName = new Map(verified.map((asset) => [asset.name, asset]));
  for (const asset of verified) {
    if (!asset.name.endsWith('.sha256')) continue;
    const payloadName = asset.name.slice(0, -'.sha256'.length);
    const payload = byName.get(payloadName);
    if (!payload)
      throw new Error(`Checksum sidecar ${asset.name} has no ${payloadName} beside it in the source release`);
    authenticateComponentChecksumSidecar(readFileSync(asset.filePath, 'utf8'), payloadName, payload.sha256);
  }
  return verified;
}

function createRelease({ repo, source, tag }) {
  const notesPath = path.join(mkdtempSync(path.join(tmpdir(), 'ghostex-mirror-notes-')), 'notes.md');
  writeFileSync(notesPath, source.body ?? '');
  const args = [
    'release',
    'create',
    tag,
    '--repo',
    repo,
    '--title',
    source.name || tag,
    '--notes-file',
    notesPath,
    '--latest=false',
  ];
  if (source.isPrerelease) args.push('--prerelease');
  const result = gh(args, { allowFailure: true, write: true });
  rmSync(path.dirname(notesPath), { recursive: true, force: true });
  if (result.status === 0) return;
  if (viewRelease({ repo, tag, write: true }).exists) return;
  throw new Error(
    `Could not create ${repo} release ${tag}: ${(result.stderr || result.stdout || 'unknown error').trim()}`
  );
}

function uploadAsset({ asset, repo, tag }) {
  const result = gh(['release', 'upload', tag, asset.filePath, '--repo', repo], { allowFailure: true, write: true });
  if (result.status === 0) return;
  const remote = (viewRelease({ repo, tag, write: true }).assets ?? []).find(
    (candidate) => candidate.name === asset.name
  );
  if (remote && sameBytes(remote, asset)) return;
  throw new Error(
    `Could not upload ${asset.name} to ${repo} ${tag}: ${(result.stderr || result.stdout || 'unknown error').trim()}`
  );
}

async function main() {
  const options = parseArguments(process.argv.slice(2));
  const prefix = options.dryRun ? 'DRY-RUN ' : '';
  const source = viewRelease({ repo: options.from, tag: options.tag });
  if (!source.exists) throw new Error(`${options.from} has no release ${options.tag}`);
  if (source.isDraft) throw new Error(`${options.from} release ${options.tag} is a draft; publish it before mirroring`);
  if (source.assets.length === 0) throw new Error(`${options.from} release ${options.tag} has no assets`);
  const destination = viewRelease({ repo: options.to, tag: options.tag, write: true });
  const plan = planMirror({ source, destination });

  process.stdout.write(
    `${prefix}MIRROR ${options.from} -> ${options.to} ${options.tag} (${source.assets.length} asset(s))\n`
  );
  if (plan.createRelease) process.stdout.write(`${prefix}CREATE ${options.to} release ${options.tag} (not latest)\n`);
  for (const asset of plan.noops) process.stdout.write(`NO-OP ${asset.name} ${normalizedRemoteDigest(asset) || ''}\n`);
  for (const asset of plan.uploads)
    process.stdout.write(`${prefix}UPLOAD ${asset.name} ${normalizedRemoteDigest(asset) || ''}\n`);
  if (options.dryRun) return;
  if (!plan.createRelease && plan.uploads.length === 0) {
    process.stdout.write(`UP-TO-DATE ${options.to} ${options.tag}\n`);
    return;
  }

  const assetDir = mkdtempSync(path.join(tmpdir(), `ghostex-mirror-${options.tag.slice(0, 40)}-`));
  try {
    /* Download every asset, not only the missing ones, so the sidecar check always sees its payload. */
    gh(['release', 'download', options.tag, '--repo', options.from, '--dir', assetDir, '--clobber']);
    const verified = verifyDownloadedAssets({ assetDir, assets: source.assets });
    const localByName = new Map(verified.map((asset) => [asset.name, asset]));
    if (plan.createRelease) createRelease({ repo: options.to, source, tag: options.tag });
    for (const asset of plan.uploads) {
      uploadAsset({ asset: localByName.get(asset.name), repo: options.to, tag: options.tag });
      process.stdout.write(`UPLOADED ${asset.name}\n`);
    }
  } finally {
    if (existsSync(assetDir)) rmSync(assetDir, { recursive: true, force: true });
  }

  const mirrored = viewRelease({ repo: options.to, tag: options.tag, write: true });
  const mirroredByName = new Map((mirrored.assets ?? []).map((asset) => [asset.name, asset]));
  for (const asset of source.assets) {
    const remote = mirroredByName.get(asset.name);
    if (!remote || Number(remote.size) !== Number(asset.size)) {
      throw new Error(`${options.to} ${options.tag} is missing ${asset.name} after the mirror`);
    }
    const sourceDigest = normalizedRemoteDigest(asset);
    const remoteDigest = normalizedRemoteDigest(remote);
    if (sourceDigest && remoteDigest && sourceDigest !== remoteDigest) {
      throw new Error(`${options.to} ${options.tag} has a different digest for ${asset.name} after the mirror`);
    }
  }
  process.stdout.write(`MIRRORED ${options.to} ${options.tag}: ${mirrored.url ?? ''}\n`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
