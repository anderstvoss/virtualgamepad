# Security Policy

## Supported Versions

Security updates are provided for the latest version on the `main` branch unless otherwise stated.

## Reporting a Vulnerability

Please do not report security vulnerabilities through public GitHub issues.

Use GitHub private vulnerability reporting if enabled, or contact the maintainer directly.

Include:

- Affected version or commit
- Reproduction steps
- Impact
- Suggested fix, if known

## Defensive Posture

This repo enforces:

- Pre-commit secret scanning (gitleaks) plus a suite of custom blockers for
  env files, private keys, credentials, local paths, private IPs, cloud
  storage URIs, and binary artifacts
- Pre-push: gitleaks full-tree scan + tracked-file blocker + local-paths
  guard + `cargo deny check` + `cargo audit`
- Weekly full-history gitleaks scan in CI
- Dependabot alerts and automated security updates
- Codeowner review required on `.github/`, security docs, and dependency
  manifests

Linux-applicable CI gates run on developer machines via pre-commit and
pre-push; GitHub Actions covers only macOS and Windows compatibility.