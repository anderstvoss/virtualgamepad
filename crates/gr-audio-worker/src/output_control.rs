//! Private operation 8. Existing startup, observation, counter and close
//! messages are also used by older clients and keep their original shapes.
use gr_curated_controllers::usb_personality::outputs::WorkerOutputs;

pub(crate) const TAG: u8 = 8;
pub(crate) const VERSION: u8 = 1;
pub(crate) const LIVE: u8 = 0;
pub(crate) const FINAL: u8 = 1;

pub(crate) fn response(generation: u64, mode: u8, outputs: &WorkerOutputs) -> Vec<u8> {
    let mut body = generation.to_le_bytes().to_vec();
    body.extend([VERSION, mode]);
    body.extend(outputs.encode());
    body
}
