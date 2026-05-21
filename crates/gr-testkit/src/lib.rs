//! Testing utilities and fixture loading for `virtualgamepad`.

pub mod assertions;
pub mod builders;
pub mod fakes;
pub mod fixtures;
pub mod harness;
pub mod proptest_strategies;
pub mod recorder;

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {}
}
