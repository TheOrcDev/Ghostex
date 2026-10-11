import path from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * CDXC:Release 2026-09-16 DECISION:
 * User: "I don't want these kinds of releases [the code-server-<identity> and cef-<version> component tags] to show up in the releases list of Ghostex itself."
 * On-demand runtime components (CEF and the Code-tab editor payloads, every platform) are published to this separate public repository, never to maddada/Ghostex.
 * The nine component releases that already exist in maddada/Ghostex stay there because shipped installs download from them; they are deleted by hand only once no supported install references them.
 * This constant is the only place the components repository is spelled out: the shell and PowerShell scripts and the workflows read it through `GHOSTEX_COMPONENTS_REPO` (resolved by running this file), and every sealed manifest records it per component so old manifests keep pointing at the repository they were sealed with.
 * SEE-ALSO: tooling/release-gpui/publish-component.mjs, tooling/release-gpui/on-demand-manifest.mjs, tooling/release-gpui/mirror-component-release.mjs, apps/desktop/src/component_store/, tooling/release-gpui/components-repo-setup.md.
 */
export const COMPONENTS_GITHUB_REPO = 'maddada/ghostex-components';

/* The app release repository: `v<version>` releases, customer downloads, the version-scoped manifest `assets`. */
export const APP_RELEASE_GITHUB_REPO = 'maddada/Ghostex';

/* Where every component published before 2026-09-16 lives; read-only from now on. */
export const LEGACY_COMPONENTS_GITHUB_REPO = APP_RELEASE_GITHUB_REPO;

export const COMPONENTS_GITHUB_REPO_ENV = 'GHOSTEX_COMPONENTS_REPO';

/*
 * `github.token` cannot write releases in another repository, so every step
 * that creates a component release or uploads a component asset authenticates
 * with this fine-grained PAT (Contents: read and write on the components
 * repository). Read-only probes of public releases keep using `github.token`.
 */
export const COMPONENTS_GITHUB_TOKEN_ENV = 'COMPONENTS_GITHUB_TOKEN';

const githubRepoPattern = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/u;

export function componentsGithubRepo(env = process.env) {
  const configured = env[COMPONENTS_GITHUB_REPO_ENV];
  if (configured === undefined || configured === '') return COMPONENTS_GITHUB_REPO;
  if (!githubRepoPattern.test(configured)) {
    throw new Error(`${COMPONENTS_GITHUB_REPO_ENV} must have owner/repository form, got ${JSON.stringify(configured)}`);
  }
  return configured;
}

/* The repositories a component tag may live in, most authoritative first, without duplicates. */
/** @param {{ component?: { githubRepo?: string }, env?: NodeJS.ProcessEnv }} [options] */
export function componentReleaseRepos({ component, env = process.env } = {}) {
  const repos = [component?.githubRepo, componentsGithubRepo(env), LEGACY_COMPONENTS_GITHUB_REPO];
  return [...new Set(repos.filter((repo) => typeof repo === 'string' && repo.length > 0))];
}

/* Environment for `gh` calls that write to the components repository. */
export function componentsGithubEnv(env = process.env) {
  const token = env[COMPONENTS_GITHUB_TOKEN_ENV];
  if (!token) return { ...env };
  return { ...env, GH_TOKEN: token, GITHUB_TOKEN: token };
}

export function requireComponentsGithubToken(env = process.env) {
  if (env[COMPONENTS_GITHUB_TOKEN_ENV]) return env[COMPONENTS_GITHUB_TOKEN_ENV];
  throw new Error(
    `${COMPONENTS_GITHUB_TOKEN_ENV} is not configured; publishing to ${componentsGithubRepo(env)} needs the fine-grained PAT ` +
      'described in tooling/release-gpui/components-repo-setup.md (github.token cannot write releases in another repository).'
  );
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.stdout.write(`${componentsGithubRepo()}\n`);
}
