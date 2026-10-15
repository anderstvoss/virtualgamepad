# Brief 12: Windows CI crate download failures (#150)

Read `ground-rules.md` first.

**Lane:** Fixes. **Blocked by:** **D8**, Anders's explicit OK for this `.github/` change. Do nothing in `.github/` before that comment exists on #150.
**Why it matters:** "Rust build and test (windows-latest)" is a required check; `CRYPT_E_REVOCATION_OFFLINE` failures block merges on every lane above.

## Work

1. Confirm the failure signature from recent failed runs (schannel revocation during crate download, before compilation). Separate transport failures from real build failures.
2. In the Windows job only:
   - cache `~/.cargo/registry` and `~/.cargo/git` keyed on `Cargo.lock` and the toolchain (`Swatinem/rust-cache` or `actions/cache`, pinned by SHA like the repo's other actions);
   - add a bounded retry (for example 3 attempts with backoff) around `cargo fetch --locked` before the build step.
3. Never set `CARGO_HTTP_CHECK_REVOCATION=false`, never use an insecure mirror, never mark the job non-required. Keep the harden-runner egress allowlist; add only the hosts the cache action needs, and list them in the PR.
4. Show a cold-cache and a warm-cache run in the PR.

## Done when

Merged with Anders's approval, #150 closed with the run links.
