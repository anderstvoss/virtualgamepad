//! YAML fixture loading primitives.

mod backend_trace;
mod input_frame;
mod plan_snapshot;
mod reverse_event;
mod schema;
mod session_scenario;

pub use schema::{FixtureEnvelope, FixtureError, load_fixture};
