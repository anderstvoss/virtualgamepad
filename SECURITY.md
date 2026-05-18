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

- GitHub secret scanning + push protection
- Branch protection on `main` (signed commits, required reviews, required status checks)
- Pre-commit secret scanning (gitleaks) and pattern blockers
- Supply-chain scanning (cargo-deny, cargo-audit, dependency-review-action)
- OpenSSF Scorecard
- Egress-blocked CI runners (step-security/harden-runner)