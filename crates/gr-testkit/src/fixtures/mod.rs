//! YAML fixture loading primitives.

mod backend_inventory;
mod backend_trace;
mod input_frame;
mod plan_snapshot;
mod reverse_event;
mod schema;
mod session_scenario;

pub use backend_inventory::{BackendInventory, BackendInventoryFixture, decode_backend_inventory};
pub use backend_trace::{
    BackendTrace, BackendTraceFixture, BackendTracePayload, BackendTraceStep, TraceDirection,
    TraceOperation, decode_backend_trace,
};
pub use input_frame::{
    InputDeltaFixture, InputFrameFixture, decode_input_delta, decode_input_frame,
};
pub use plan_snapshot::{PlanOutcome, PlanSnapshotFixture, decode_plan_snapshot};
pub use reverse_event::{ReverseEventFixture, decode_reverse_event};
pub use schema::{FixtureDocument, FixtureEnvelope, FixtureError, load_fixture};
pub use session_scenario::{
    ScenarioBackend, ScenarioFailure, ScenarioStep, SessionScenario, SessionScenarioFixture,
    decode_session_scenario,
};
