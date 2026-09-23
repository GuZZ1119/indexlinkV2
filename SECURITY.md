# Security Policy

## Supported scope

IndexLink V2.1 is a single-user, local-first application. The supported security boundary is:

- the Rust API binds to `127.0.0.1` by default;
- Docker publishes the API only on host loopback;
- SQLite, API credentials, and OpenD remain on the user's own machine;
- broker integration is paper-only and requires an explicit user action;
- AI features are optional, read-only helpers invoked manually.

The current API has no account authentication. Exposing it to a LAN, a public IP, a cloud load balancer, or an untrusted reverse proxy is **unsupported and unsafe**.

## Credential handling

- `.env`, SQLite files, coverage output, IDE state, and local demo media are ignored by Git.
- AI keys entered in Advanced Lab are held only in current-process memory, are never returned by the API, and are cleared on process exit.
- Provider error types and logs are designed not to include keys or credential-bearing URLs.
- OpenD hosts must be literal loopback addresses; a Lab market-data session never enables broker capability.
- Do not paste real API keys into issues, screenshots, test fixtures, or `CHANGE_LOG.md`.

## Network and parser limits

- HTTP request bodies are limited to 1 MiB.
- RSS responses are streamed into a buffer capped at 1 MiB before XML parsing.
- AI JSON extraction is string- and escape-aware; parse failures log only safe metadata, not the raw model response.
- Remote AI endpoints are selected by server-side provider profiles and require HTTPS.

## Dependency status (2026-09-23)

- `pnpm audit --prod`: **0 advisories** after removing the shadcn CLI runtime dependency and upgrading React Router, Vite, and Tailwind.
- `cargo audit`: one lock-file advisory remains: `RUSTSEC-2023-0071` for `rsa 0.9.10`, with no fixed release. `cargo tree -i rsa` has no active path in the supported SQLite build; it is retained in Cargo's resolved SQLx metadata and is not linked into the current target. This is an accepted V2.1 lock-only risk, not permission to enable MySQL.
- Former runtime advisories for `quick-xml 0.37.5` and `rustls 0.23.40` were remediated by upgrading to `quick-xml 0.42.0` and `rustls 0.23.45` or later compatible lock revisions.
- Former warnings for `event-listener 5.4.1` and yanked `spin 0.9.8` were removed by refreshing compatible dependencies.

Re-run both audits before every public release. Do not silently suppress the remaining RustSec advisory; keep this rationale current until SQLx no longer resolves the affected package.

## Known limitations

The following are product limitations, not hidden assurances:

1. There is no authentication, authorization, CSRF token, or remote-session model.
2. Browser-to-backend traffic is plain HTTP because the supported deployment is same-machine loopback.
3. The server does not yet include general per-route rate limiting.
4. Research data remains subject to the source provider's license and redistribution terms.
5. This review is not an independent penetration test.

If remote access is added later, it must be a separate release with authentication, TLS, origin/CSRF controls, rate limits, secure session storage, and a fresh threat model.

## Reporting a vulnerability

Please do not open a public issue containing exploit details, credentials, personal data, or a working attack against a user installation. Contact the repository maintainer privately through the security-reporting channel shown on the GitHub repository. Include:

- affected commit and platform;
- prerequisites and impact;
- minimal reproduction steps;
- whether credentials or user data may have been exposed;
- a suggested mitigation, if available.

Reports should be acknowledged before public disclosure. Never test against systems you do not own or have explicit permission to assess.
