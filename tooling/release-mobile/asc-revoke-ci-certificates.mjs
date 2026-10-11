#!/usr/bin/env node
/*
 * CDXC:Release 2026-10-11 DECISION:
 * User: revoke the CI-made development certificates before the iOS archive and the one the run created afterwards (also on failure), so the account's certificate limit never fills. Only revoke certificates whose name shows the API made them for CI; never touch certificates named after a person, distribution certificates, or anything that cannot be positively identified as CI-made. Log what is revoked.
 *
 * CDXC:Release 2026-10-11 WHY:
 * `xcodebuild -allowProvisioningUpdates` with the App Store Connect API key finds no development identity on a fresh runner, so every run creates an "Apple Development: Created via API (<API key ID>)" certificate whose private key dies with the runner. The 10.19.0 TestFlight run (38025945451) failed on "Your account has reached the maximum number of certificates". A certificate is CI-made only when all of these hold: its type is DEVELOPMENT or IOS_DEVELOPMENT, and the X.509 subject inside its content (not only the portal's display name) has a CN of exactly "Apple Development: Created via API (" + 10 uppercase letters or digits + ")" and OU = the team. The parenthesised value is the key ID, not the team: run 38099870415 kept every certificate when the CN was matched against the team ID, and older keys' certificates carry their own IDs.
 *
 * Usage: asc-revoke-ci-certificates.mjs <teamId> [--dry-run]
 * Env: GHOSTEX_ASC_KEY_PATH, GHOSTEX_ASC_KEY_ID, GHOSTEX_ASC_ISSUER_ID (the workflow's key step).
 */

import { X509Certificate } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { ascClientFromEnv } from "./asc-api.mjs";

const DEVELOPMENT_TYPES = new Set(["DEVELOPMENT", "IOS_DEVELOPMENT"]);
/** The parenthesised value is the ID of the API key that made the certificate. */
const CI_COMMON_NAME = /^Apple Development: Created via API \([A-Z0-9]{10}\)$/;

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
  const subject = `CN "${fields.CN ?? ""}" OU "${fields.OU ?? ""}"`;
  if (!CI_COMMON_NAME.test(fields.CN ?? "")) {
    return { ci: false, subject, reason: "CN is not a Created via API one" };
  }
  if (fields.OU !== teamId) {
    return { ci: false, subject, reason: `OU is not team ${teamId}` };
  }
  return { ci: true, subject };
}

async function listDevelopmentCertificates(client) {
  const fields =
    "name,certificateType,serialNumber,expirationDate,certificateContent";
  const { data } = await client.list(
    `/v1/certificates?filter[certificateType]=${[...DEVELOPMENT_TYPES].join(",")}&fields[certificates]=${fields}&limit=200`,
  );
  return data;
}

async function main(argv) {
  const teamId = argv[0];
  const dryRun = argv.includes("--dry-run");
  if (!/^[A-Z0-9]{10}$/.test(teamId ?? ""))
    throw new Error(
      `usage: asc-revoke-ci-certificates.mjs <teamId> [--dry-run]`,
    );
  const client = ascClientFromEnv();
  const certificates = await listDevelopmentCertificates(client);
  let revoked = 0;
  for (const resource of certificates) {
    const { attributes = {} } = resource;
    const label = `${resource.id} "${attributes.name ?? ""}" serial ${attributes.serialNumber ?? "?"} expires ${attributes.expirationDate ?? "?"}`;
    const verdict = classifyCertificate(resource, teamId);
    if (!verdict.ci) {
      console.log(
        `kept ${label}${verdict.subject ? ` ${verdict.subject}` : ""}: ${verdict.reason}`,
      );
      continue;
    }
    if (dryRun) {
      console.log(`would revoke ${label} ${verdict.subject}`);
      continue;
    }
    await client.request(
      "DELETE",
      `/v1/certificates/${encodeURIComponent(resource.id)}`,
    );
    revoked += 1;
    console.log(`revoked ${label} ${verdict.subject}`);
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
