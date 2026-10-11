#!/usr/bin/env node
/*
 * CDXC:Release 2026-10-11 DECISION:
 * User: revoke the CI-made development certificates before the iOS archive and the one the run created afterwards (also on failure), so the account's certificate limit never fills. Only revoke certificates whose name shows the API made them for CI; never touch certificates named after a person, distribution certificates, or anything that cannot be positively identified as CI-made. Log what is revoked.
 *
 * CDXC:Release 2026-10-11 WHY:
 * `xcodebuild -allowProvisioningUpdates` with the App Store Connect API key finds no development identity on a fresh runner, so every run creates an "Apple Development: Created via API (<team>)" certificate whose private key dies with the runner. The 10.19.0 TestFlight run (38025945451) failed on "Your account has reached the maximum number of certificates". A certificate is CI-made only when all of these hold: its type is DEVELOPMENT or IOS_DEVELOPMENT, and the X.509 subject inside its content (not only the portal's display name) has exactly that CN and OU = the team.
 *
 * Usage: asc-revoke-ci-certificates.mjs <teamId> [--dry-run]
 * Env: GHOSTEX_ASC_KEY_PATH, GHOSTEX_ASC_KEY_ID, GHOSTEX_ASC_ISSUER_ID (the workflow's key step).
 */

import { X509Certificate, sign } from "node:crypto";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const API = "https://api.appstoreconnect.apple.com";
const DEVELOPMENT_TYPES = new Set(["DEVELOPMENT", "IOS_DEVELOPMENT"]);

function base64url(value) {
  return Buffer.from(value).toString("base64url");
}

/** App Store Connect API token: ES256 JWT, valid for 10 minutes (the API allows 20). */
export function ascToken({
  keyPem,
  keyId,
  issuerId,
  now = Math.floor(Date.now() / 1000),
}) {
  const header = base64url(
    JSON.stringify({ alg: "ES256", kid: keyId, typ: "JWT" }),
  );
  const payload = base64url(
    JSON.stringify({
      iss: issuerId,
      iat: now,
      exp: now + 600,
      aud: "appstoreconnect-v1",
    }),
  );
  const signature = sign("sha256", Buffer.from(`${header}.${payload}`), {
    key: keyPem,
    dsaEncoding: "ieee-p1363",
  });
  return `${header}.${payload}.${signature.toString("base64url")}`;
}

/** The subject's attributes, e.g. { CN: '…', OU: '…' }. A repeated attribute makes the subject unreadable. */
function subjectFields(x509) {
  const fields = {};
  for (const line of x509.subject.split("\n")) {
    const at = line.indexOf("=");
    if (at <= 0) return null;
    const key = line.slice(0, at);
    if (key in fields) return null;
    fields[key] = line.slice(at + 1);
  }
  return fields;
}

/**
 * Whether one `certificates` resource is a CI-made development certificate, and why not when it is
 * not. Anything this cannot read is kept.
 */
export function classifyCertificate(resource, teamId) {
  const attributes = resource?.attributes ?? {};
  if (!DEVELOPMENT_TYPES.has(attributes.certificateType)) {
    return {
      ci: false,
      reason: `type ${attributes.certificateType ?? "unknown"}`,
    };
  }
  if (!attributes.certificateContent)
    return { ci: false, reason: "no certificate content" };
  let fields;
  try {
    fields = subjectFields(
      new X509Certificate(Buffer.from(attributes.certificateContent, "base64")),
    );
  } catch (error) {
    return { ci: false, reason: `unreadable certificate (${error.message})` };
  }
  if (!fields) return { ci: false, reason: "unreadable subject" };
  if (fields.CN !== `Apple Development: Created via API (${teamId})`) {
    return { ci: false, reason: `subject CN "${fields.CN ?? ""}"` };
  }
  if (fields.OU !== teamId)
    return { ci: false, reason: `subject OU "${fields.OU ?? ""}"` };
  return { ci: true, cn: fields.CN };
}

async function ascRequest(token, method, url) {
  const response = await fetch(
    url.startsWith("https://") ? url : `${API}${url}`,
    {
      method,
      headers: { Authorization: `Bearer ${token}` },
    },
  );
  if (!response.ok) {
    throw new Error(
      `${method} ${url} answered ${response.status}: ${(await response.text()).slice(0, 500)}`,
    );
  }
  return response.status === 204 ? null : response.json();
}

async function listDevelopmentCertificates(token) {
  const fields =
    "name,certificateType,serialNumber,expirationDate,certificateContent";
  let url = `/v1/certificates?filter[certificateType]=${[...DEVELOPMENT_TYPES].join(",")}&fields[certificates]=${fields}&limit=200`;
  const resources = [];
  while (url) {
    const page = await ascRequest(token, "GET", url);
    resources.push(...(page.data ?? []));
    url = page.links?.next ?? null;
  }
  return resources;
}

async function main(argv) {
  const teamId = argv[0];
  const dryRun = argv.includes("--dry-run");
  if (!/^[A-Z0-9]{10}$/.test(teamId ?? ""))
    throw new Error(
      `usage: asc-revoke-ci-certificates.mjs <teamId> [--dry-run]`,
    );
  const {
    GHOSTEX_ASC_KEY_PATH: keyPath,
    GHOSTEX_ASC_KEY_ID: keyId,
    GHOSTEX_ASC_ISSUER_ID: issuerId,
  } = process.env;
  if (!keyPath || !keyId || !issuerId)
    throw new Error(
      "GHOSTEX_ASC_KEY_PATH, GHOSTEX_ASC_KEY_ID and GHOSTEX_ASC_ISSUER_ID are required",
    );

  const token = ascToken({
    keyPem: readFileSync(keyPath, "utf8"),
    keyId,
    issuerId,
  });
  const certificates = await listDevelopmentCertificates(token);
  let revoked = 0;
  for (const resource of certificates) {
    const { attributes = {} } = resource;
    const label = `${resource.id} "${attributes.name ?? ""}" serial ${attributes.serialNumber ?? "?"} expires ${attributes.expirationDate ?? "?"}`;
    const verdict = classifyCertificate(resource, teamId);
    if (!verdict.ci) {
      console.log(`kept ${label}: ${verdict.reason}`);
      continue;
    }
    if (dryRun) {
      console.log(`would revoke ${label} (${verdict.cn})`);
      continue;
    }
    await ascRequest(
      token,
      "DELETE",
      `/v1/certificates/${encodeURIComponent(resource.id)}`,
    );
    revoked += 1;
    console.log(`revoked ${label} (${verdict.cn})`);
  }
  console.log(
    `${dryRun ? "Dry run: " : ""}${revoked} CI development certificate(s) revoked, ${certificates.length} development certificate(s) seen.`,
  );
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error.message);
    process.exit(1);
  });
}
