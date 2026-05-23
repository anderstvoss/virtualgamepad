#![forbid(unsafe_code)]

//! Forward and reverse translators for `virtualgamepad`.
//!
//! Trait shapes pinned in the Phase 6 prep PR; per-family translator
//! bodies + real HID descriptor bytes land in Phase 6 itself.
//!
//! # Translator semantics
//!
//! Translators are descriptor-driven: per-profile byte/bit mappings
//! are defined by the live [`gr_profiles::DescriptorTemplate`]
//! referenced from [`gr_runtime_model::PreparedTranslationContext`],
//! not duplicated in code. Phase 6 implementations consult the
//! descriptor through the prepared context; changing a mapping means
//! updating the descriptor, not the translator code.

use gr_backend_api::{BackendFrame, BackendReverseEvent};
use gr_core::ProfileInputFrame;
use gr_runtime_model::{
    ControllerOutputCommand, PreparedTranslationContext, SessionPlan, TranslatorFamily,
};
use smallvec::SmallVec;
use thiserror::Error;

/// Errors raised by forward or reverse translation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TranslationError {
    #[error(
        "profile family `{family:?}` has no registered translator for backend level `{level:?}`"
    )]
    NoTranslatorRegistered {
        family: TranslatorFamily,
        level: gr_core::BackendLevel,
    },
    #[error("input frame does not satisfy the profile contract: {reason}")]
    InvalidInput { reason: String },
    #[error("reverse event payload is malformed: {reason}")]
    InvalidReverseEvent { reason: String },
    #[error("descriptor template is unavailable for the selected fidelity tier")]
    DescriptorUnavailable,
    #[error("translation produced a frame that violates the descriptor: {reason}")]
    DescriptorViolation { reason: String },
}

/// Reusable per-session scratch area for forward translators.
///
/// The session actor owns one of these and passes it by mutable
/// reference to every `translate()` call so steady-state translation
/// avoids per-frame allocation.
#[derive(Debug, Default, Clone)]
pub struct TranslationScratch {
    pub bytes: Vec<u8>,
}

impl TranslationScratch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear the scratch buffer without releasing its allocation.
    pub fn clear(&mut self) {
        self.bytes.clear();
    }
}

pub trait ForwardTranslator: Send + Sync {
    fn family(&self) -> TranslatorFamily;

    /// Translate a profile input frame into a backend frame using the
    /// prepared session context and a reusable scratch buffer.
    ///
    /// # Errors
    ///
    /// Returns [`TranslationError::InvalidInput`] if the input violates
    /// the profile contract, or [`TranslationError::DescriptorViolation`]
    /// if the produced frame would not match the selected descriptor.
    fn translate(
        &self,
        input: &ProfileInputFrame,
        ctx: &PreparedTranslationContext,
        out: &mut TranslationScratch,
    ) -> Result<BackendFrame, TranslationError>;
}

pub trait ReverseTranslator: Send + Sync {
    fn family(&self) -> TranslatorFamily;

    /// Decode a backend reverse event into one or more
    /// [`ControllerOutputCommand`] values.
    ///
    /// # Errors
    ///
    /// Returns [`TranslationError::InvalidReverseEvent`] if the event
    /// payload cannot be decoded against the active descriptor.
    fn translate_reverse(
        &self,
        event: &BackendReverseEvent,
        ctx: &PreparedTranslationContext,
        out: &mut SmallVec<[ControllerOutputCommand; 4]>,
    ) -> Result<(), TranslationError>;
}

/// Closed v1 translator registry — mirrors the
/// `gr-profiles::CapabilityRegistry` pattern: zero-sized facade over
/// `&'static` data populated in Phase 6 with the per-family
/// implementations. Future plugin-style extension is intentionally not
/// supported in v1.
#[derive(Debug, Default)]
pub struct TranslatorRegistry {
    _private: (),
}

impl TranslatorRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }

    /// Resolve the forward translator for `(family, level)`. Returns
    /// `None` if no translator is registered for the pair.
    #[must_use]
    pub fn forward(
        &self,
        _family: TranslatorFamily,
        _level: gr_core::BackendLevel,
    ) -> Option<&'static dyn ForwardTranslator> {
        None
    }

    /// Resolve the reverse translator for `family`. Returns `None` if
    /// no translator is registered.
    #[must_use]
    pub fn reverse(&self, _family: TranslatorFamily) -> Option<&'static dyn ReverseTranslator> {
        None
    }
}

/// Build the per-session [`PreparedTranslationContext`] from a
/// [`SessionPlan`].
///
/// Phase 6 implements the body; the prep PR pins the signature so
/// downstream crates (`gr-session`) can be written against a stable
/// contract.
///
/// # Errors
///
/// Returns [`TranslationError::NoTranslatorRegistered`] when no
/// translator is registered for the plan's family + level.
///
/// # Panics
///
/// Stub implementation panics via `unimplemented!()`. Phase 6 replaces
/// the body.
pub fn prepared_translation_context(
    _plan: &SessionPlan,
    _registry: &TranslatorRegistry,
) -> Result<PreparedTranslationContext, TranslationError> {
    unimplemented!("Phase 6 prepared-context construction")
}

#[cfg(test)]
mod tests {
    use super::{TranslationError, TranslationScratch, TranslatorRegistry};
    use gr_core::BackendLevel;
    use gr_runtime_model::TranslatorFamily;

    #[test]
    fn registry_default_is_empty() {
        let registry = TranslatorRegistry::new();
        assert!(
            registry
                .forward(TranslatorFamily::DualSense, BackendLevel::Hid)
                .is_none()
        );
        assert!(registry.reverse(TranslatorFamily::DualSense).is_none());
    }

    #[test]
    fn translation_error_display_is_human_readable() {
        let error = TranslationError::DescriptorUnavailable;
        assert!(error.to_string().contains("descriptor template"));
    }

    #[test]
    fn translation_scratch_clears_without_releasing() {
        let mut scratch = TranslationScratch::new();
        scratch.bytes.extend_from_slice(&[1, 2, 3, 4]);
        let before = scratch.bytes.capacity();
        scratch.clear();
        assert!(scratch.bytes.is_empty());
        assert_eq!(scratch.bytes.capacity(), before);
    }
}
