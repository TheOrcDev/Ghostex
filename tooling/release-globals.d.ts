// Fields the release tooling attaches to thrown errors (tooling/release-gpui/retry.mjs,
// publish-homebrew-cask.mjs) so callers can tell a retryable failure from a final one.
interface Error {
  ghostexClassification?: unknown;
  ghostexHttpStatus?: number;
  ghostexRetryAttempts?: number;
  ghostexRetryLabel?: string;
}
