import { readFileSync } from 'node:fs';
import { describe, expect, test } from 'vitest';

// The cask renderer went with the local release driver (deleted 2026-10-01); release-final-verify still
// checks the published tap cask with validateGhostexCask in tooling/release-shared.mjs.
const releaseSharedSource = readFileSync(new URL('../../../tooling/release-shared.mjs', import.meta.url), 'utf8');

function sourceFrom(source: string, start: string): string {
  const startIndex = source.indexOf(start);
  expect(startIndex).toBeGreaterThanOrEqual(0);
  return source.slice(startIndex);
}

describe('Ghostex CLI command wrappers', () => {
  test('the published Homebrew cask check requires wrappers instead of CLI binary aliases', () => {
    /*
     * CDXC:Cli 2026-06-12-09:31:
     * Release automation must not reintroduce Homebrew binary stanzas for
     * ghostex/gx, because those stanzas create symlinks back into Ghostex.app.
     */
    const validator = sourceFrom(releaseSharedSource, 'export function validateGhostexCask');

    // Homebrew 7.0.6 shape (CDXC:Release 2026-09-21): command_wrapper stanzas and *_steps blocks.
    expect(validator).toContain('\'command_wrapper "ghostex", content:\'');
    expect(validator).toContain('\'command_wrapper "gx", content:\'');
    expect(validator).toContain('\'exec "#{appdir}/ghostex.app/Contents/Resources/CLI/ghostex" "$@"\'');
    expect(validator).toContain("'depends_on macos: :ventura'");
    expect(validator).toContain("['preflight_steps', 'postflight_steps', 'uninstall_preflight_steps']");
    expect(validator).toContain('Ghostex cask must install wrapper files, not Homebrew binary aliases.');
  });
});
