#![forbid(unsafe_code)]

//! Linux UHID provider for `virtualgamepad`.
//!
//! Phase 9 implementation is still pending; the crate root stays
//! target-scoped so feature wiring at the workspace root can reference
//! it cleanly without leaking Linux assumptions into other platforms.

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {}
}
