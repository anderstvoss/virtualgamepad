//! Compiles as a standalone downstream using only the root application API.
use std::sync::{Arc, Mutex};
use virtualgamepad::{
    AudioAccess, AudioError, AudioExposure, AudioOptions, ComponentKind, ControllerError,
    CreationOptions, DualSenseController, DualShock4Controller, RealizationId, SwitchProController,
    Xbox360Controller,
};

pub enum Controller {
    Sony(DualSenseController),
    Ds4(DualShock4Controller),
    Xbox(Xbox360Controller),
    Switch(SwitchProController),
}
impl Controller {
    pub fn cycle(&mut self) -> Result<(), ControllerError> {
        match self {
            Self::Sony(c) => c.service(&mut |_| {}),
            Self::Ds4(c) => c.service(&mut |_| {}),
            Self::Xbox(c) => c.service(&mut |_| {}),
            Self::Switch(c) => c.service(&mut |_| {}),
        }
    }
    pub fn pcm_batch(&mut self) -> Result<usize, AudioError> {
        let audio = match self {
            Self::Sony(c) => c.audio(),
            Self::Ds4(c) => c.audio(),
            Self::Xbox(c) => c.audio(),
            Self::Switch(_) => None,
        };
        if let Some(audio) = audio {
            let health = audio.diagnostics();
            if health.is_closed() {
                return Err(AudioError::Closed);
            }
            if audio.endpoints()[0].access() == AudioAccess::Samples {
                return Ok(audio.read_playback(&mut [0; 128 * 4])?.frames);
            }
        }
        Ok(0)
    }
    pub fn close(&mut self) {
        match self {
            Self::Sony(c) => c.close(),
            Self::Ds4(c) => c.close(),
            Self::Xbox(c) => c.close(),
            Self::Switch(c) => c.close(),
        }
    }
}

pub fn service_and_pcm(controller: &mut Controller) -> Result<usize, Box<dyn std::error::Error>> {
    controller.cycle()?;
    Ok(controller.pcm_batch()?)
}

pub fn threaded_batch(
    controller: Arc<Mutex<Controller>>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let pcm_owner = controller.clone();
    let pcm = std::thread::spawn(move || pcm_owner.lock().unwrap().pcm_batch());
    let service = controller.lock().unwrap().cycle();
    let samples = pcm.join().unwrap();
    service?;
    Ok(samples?)
}

pub fn inspect(controller: &DualSenseController) {
    for component in controller.association().components() {
        match component.kind() {
            ComponentKind::Input => {
                let topology = component.surface().unwrap().input_topology();
                for stick in topology.sticks() {
                    let _ = (stick.id(), stick.x(), stick.y());
                }
            }
            ComponentKind::Audio => {
                let endpoint = component.audio_endpoint().unwrap();
                let _ = (endpoint.host().pipewire_node(), endpoint.host().alsa_pcm());
            }
            _ => {}
        }
    }
}

pub fn creation_options() -> CreationOptions {
    CreationOptions::new(RealizationId::LINUX_UHID_USB).with_audio(
        AudioOptions::new(AudioExposure::Emulated).with_playback_access(AudioAccess::Samples),
    )
}
