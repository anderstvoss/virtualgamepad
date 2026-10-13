# Changelog

All notable changes to this project are documented here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Security

- Enforce `// SAFETY:` comments on every `unsafe` block
  (`clippy::undocumented_unsafe_blocks = "deny"`).
- Lint Python helpers with ruff (pyflakes + bandit rules) and run their
  unit tests in CI.
- Extend CodeQL to scan GitHub Actions workflows.
- Add harden-runner egress blocking to the full-history gitleaks workflow.
- Require a full-history secret scan before changing repository
  visibility; document Actions token, fork-approval and tag-protection
  settings.
