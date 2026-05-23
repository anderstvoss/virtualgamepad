//! Standalone `kind: reverse-event` fixtures are not currently used.
//!
//! Reverse events are carried inside `kind: backend-trace` fixtures as
//! inbound steps (see `backend_trace::BackendTracePayload::ReverseEvent`).
//! The `BackendReverseEvent` payload there serializes the same shape a
//! standalone reverse-event fixture would carry, so Phase 6
//! reverse-translator tests load `backend-trace` fixtures and decode
//! the inbound steps directly.
//!
//! If a standalone fixture form is ever needed (single-event
//! regression tests, fuzzing harnesses, etc.), wire
//! `decode_reverse_event` here and add the `kind: reverse-event`
//! dispatch in [`super::schema::load_fixture`]. The existing
//! `FixtureDocument::Envelope` fallback already accepts the kind
//! today; this module just lacks a typed loader.
