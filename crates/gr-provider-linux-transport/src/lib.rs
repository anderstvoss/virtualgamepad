#![forbid(unsafe_code)]

//! Linux transport provider foundation for `virtualgamepad`.
//!
//! The real transport implementation is deferred to later phases; this
//! crate exists now so provider feature wiring and planner contracts can
//! stabilize ahead of the transport work itself.

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {}
}
