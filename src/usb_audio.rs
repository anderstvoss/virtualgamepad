//! Root error mapping only; transport, PCM and worker ownership live below root.
#[cfg(test)]
pub(crate) use gr_root_audio::integration::usb::worker_output;
macro_rules! factory {
    ($name:ident,$state:ty) => {
        pub(crate) fn $name(
            identity: [u8; 6],
            options: crate::AudioOptions,
            creation: u64,
        ) -> Result<
            (
                Box<dyn gr_curated_controllers::WorkerBridge<$state>>,
                crate::ControllerAudio,
            ),
            crate::ControllerError,
        > {
            gr_root_audio::integration::usb::$name(identity, options, creation)
                .map_err(crate::audio::open_error)
        }
    };
}
factory!(dualsense, gr_curated_controllers::DualSenseState);
factory!(dualshock4, gr_curated_controllers::DualShock4State);
factory!(xbox360, gr_curated_controllers::Xbox360State);
