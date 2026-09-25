//! Private session seam; applications never implement or construct a backend.
use crate::{AudioError, AudioRead};

pub(crate) trait Backend: Send + Sync {
    fn timings(&self) -> Vec<super::AudioStreamTiming>;
    fn read_playback(&mut self, dest: &mut [i16]) -> Result<AudioRead, AudioError>;
    fn write_microphone(&mut self, samples: &[i16]) -> Result<usize, AudioError>;
    fn flush_playback(&mut self) -> Result<(), AudioError>;
    fn flush_microphone(&mut self) -> Result<(), AudioError>;
    fn is_closed(&self) -> bool;
    fn underrun_frames(&self) -> u64;
    fn dropped_playback_frames(&self) -> u64;
    fn native_playback_underrun_frames(&self) -> Option<u64> {
        None
    }
    fn native_microphone_dropped_frames(&self) -> Option<u64> {
        None
    }
    fn native_microphone_queue_frames(&self) -> Option<(u64, u64)> {
        None
    }
    fn native_bridge_scheduling_us(&self) -> Option<(u64, u64)> {
        None
    }
    fn microphone_host_frames(&mut self) -> Result<Option<u64>, AudioError> {
        Ok(None)
    }
    fn dropped_microphone_frames(&mut self) -> Result<Option<u64>, AudioError> {
        Ok(None)
    }
    fn error(&self) -> Option<&AudioError>;
    fn failed(&self) -> bool;
    fn close(&mut self);
}

#[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
impl Backend for gr_audio_linux::Session {
    fn timings(&self) -> Vec<super::AudioStreamTiming> {
        Self::timings(self)
            .into_iter()
            .map(|t| super::AudioStreamTiming {
                endpoint: t.endpoint,
                graph_ticks: t.graph_ticks,
                tick_rate: (t.rate_num, t.rate_denom),
                observed_at_ns: t.monotonic_ns,
                estimated_graph_delay_ticks: t.delay_ticks,
                discontinuities: t.discontinuities,
                missed_graph_frames: t.missed_graph_frames,
            })
            .collect()
    }

    fn read_playback(&mut self, dest: &mut [i16]) -> Result<AudioRead, AudioError> {
        Self::read_playback(self, dest)
    }
    fn write_microphone(&mut self, samples: &[i16]) -> Result<usize, AudioError> {
        Self::write_microphone(self, samples)
    }
    fn flush_playback(&mut self) -> Result<(), AudioError> {
        Self::flush_playback(self)
    }
    fn flush_microphone(&mut self) -> Result<(), AudioError> {
        Self::flush_microphone(self)
    }
    fn is_closed(&self) -> bool {
        Self::is_closed(self)
    }
    fn underrun_frames(&self) -> u64 {
        Self::underrun_frames(self)
    }
    fn dropped_playback_frames(&self) -> u64 {
        Self::dropped_playback_frames(self)
    }
    fn error(&self) -> Option<&AudioError> {
        Self::error(self)
    }
    fn failed(&self) -> bool {
        Self::failed(self)
    }
    fn close(&mut self) {
        Self::close(self);
    }
}
