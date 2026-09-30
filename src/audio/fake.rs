//! Deterministic private I/O seam; no normal-root provider construction.
use super::{backend::Backend, *};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(crate) struct Record {
    pub failed: bool,
    pub cleanup_failure: bool,
    pub closes: usize,
    pub reads: usize,
    pub writes: usize,
    pub playback_flushes: usize,
    pub microphone_flushes: usize,
    playback: VecDeque<i16>,
    microphone: Vec<i16>,
    next_frame: u64,
}
struct Fake {
    record: Arc<Mutex<Record>>,
    access: AudioAccess,
    error: Option<AudioError>,
}
impl Fake {
    fn validate(&self, samples: usize) -> Result<(), AudioError> {
        if self.is_closed() {
            return Err(AudioError::Closed);
        }
        if self.access != AudioAccess::Samples {
            return Err(AudioError::OwnershipMismatch);
        }
        if samples % 2 != 0 {
            return Err(AudioError::InvalidSampleBuffer);
        }
        Ok(())
    }
}
impl Backend for Fake {
    fn timings(&self) -> Vec<AudioStreamTiming> {
        Vec::new()
    }
    fn read_playback(&mut self, dest: &mut [i16]) -> Result<AudioRead, AudioError> {
        self.validate(dest.len())?;
        let mut record = self.record.lock().unwrap();
        record.reads += 1;
        let frames = (dest.len() / 2).min(record.playback.len() / 2);
        let first_frame = record.next_frame;
        for value in &mut dest[..frames * 2] {
            *value = record.playback.pop_front().unwrap();
        }
        record.next_frame += frames as u64;
        Ok(AudioRead {
            frames,
            first_frame,
            discontinuity: false,
        })
    }
    fn write_microphone(&mut self, samples: &[i16]) -> Result<usize, AudioError> {
        self.validate(samples.len())?;
        let mut record = self.record.lock().unwrap();
        record.writes += 1;
        let frames = (samples.len() / 2).min(32 - record.microphone.len() / 2);
        record.microphone.extend_from_slice(&samples[..frames * 2]);
        Ok(frames)
    }
    fn flush_playback(&mut self) -> Result<(), AudioError> {
        self.validate(0)?;
        let mut record = self.record.lock().unwrap();
        record.playback_flushes += 1;
        record.playback.clear();
        Ok(())
    }
    fn flush_microphone(&mut self) -> Result<(), AudioError> {
        self.validate(0)?;
        let mut record = self.record.lock().unwrap();
        record.microphone_flushes += 1;
        record.microphone.clear();
        Ok(())
    }
    fn is_closed(&self) -> bool {
        self.record.lock().unwrap().closes != 0
    }
    fn underrun_frames(&self) -> u64 {
        7
    }
    fn dropped_playback_frames(&self) -> u64 {
        3
    }
    fn error(&self) -> Option<&AudioError> {
        self.error.as_ref()
    }
    fn failed(&self) -> bool {
        self.record.lock().unwrap().failed || self.error.is_some()
    }
    fn close(&mut self) {
        let mut record = self.record.lock().unwrap();
        if record.closes != 0 {
            return;
        }
        record.closes += 1;
        let mut errors = Vec::new();
        if record.failed {
            errors.push("synthetic audio failure");
        }
        if record.cleanup_failure {
            errors.push("synthetic audio cleanup failure");
        }
        if !errors.is_empty() {
            self.error = Some(AudioError::Backend {
                reason: errors.join("; "),
            });
        }
    }
}
pub(crate) fn open(creation: u64, access: AudioAccess) -> (ControllerAudio, Arc<Mutex<Record>>) {
    let record = Arc::new(Mutex::new(Record {
        playback: [101, -202, 303, -404].into(),
        ..Record::default()
    }));
    let format = PcmFormat::new(
        48_000,
        &[
            crate::AudioChannel::AudibleLeft,
            crate::AudioChannel::AudibleRight,
        ],
    )
    .unwrap();
    let endpoints = [
        SampleDirection::HostToController,
        SampleDirection::ControllerToHost,
    ]
    .into_iter()
    .map(|direction| AudioEndpoint {
        group: if direction == SampleDirection::HostToController {
            "playback"
        } else {
            "microphone"
        },
        clock_domain: format!("synthetic:{creation}"),
        host: AudioEndpointSelector::PipeWireNode {
            name: format!("synthetic:{creation}:{direction:?}"),
        },
        caller: None,
        format: format.clone(),
        direction,
        access,
    })
    .collect();
    (
        ControllerAudio {
            endpoints,
            limitation: "Synthetic I/O only",
            session: Box::new(Fake {
                record: record.clone(),
                access,
                error: None,
            }),
        },
        record,
    )
}

#[test]
fn sample_alignment_ownership_and_terminal_priority_are_typed() {
    let (mut samples, record) = open(1, AudioAccess::Samples);
    assert_eq!(
        samples.read_playback(&mut [0; 3]),
        Err(AudioError::InvalidSampleBuffer)
    );
    assert_eq!(
        samples.write_microphone(&[0; 3]),
        Err(AudioError::InvalidSampleBuffer)
    );
    let mut buffer = [9; 6];
    let read = samples.read_playback(&mut buffer).unwrap();
    assert_eq!(
        (read.frames, read.first_frame, read.discontinuity),
        (2, 0, false)
    );
    assert_eq!(buffer, [101, -202, 303, -404, 9, 9]);
    samples.flush_playback().unwrap();
    samples.flush_microphone().unwrap();
    samples.close();
    samples.close();
    assert_eq!(record.lock().unwrap().closes, 1);
    let retained = samples.diagnostics();
    assert!(retained.is_closed());
    assert!(!retained.failed());
    assert_eq!(
        (
            retained.underrun_frames(),
            retained.dropped_playback_frames()
        ),
        (7, 3)
    );
    assert_eq!(samples.read_playback(&mut [0; 3]), Err(AudioError::Closed));
    assert_eq!(samples.write_microphone(&[0; 3]), Err(AudioError::Closed));
    let (mut native, _) = open(2, AudioAccess::NativeClient);
    assert_eq!(
        native.read_playback(&mut [0; 2]),
        Err(AudioError::OwnershipMismatch)
    );
    assert_eq!(
        native.write_microphone(&[0; 2]),
        Err(AudioError::OwnershipMismatch)
    );
    assert_eq!(native.flush_playback(), Err(AudioError::OwnershipMismatch));
    assert_eq!(
        native.flush_microphone(),
        Err(AudioError::OwnershipMismatch)
    );
    native.close();
    assert_eq!(native.flush_playback(), Err(AudioError::Closed));
    assert_eq!(samples.diagnostics(), retained);
}

#[test]
fn rollback_preserves_typed_creation_cause_and_audio_cleanup_failure() {
    for cleanup_failure in [false, true] {
        let (session, record) = open(1, AudioAccess::Samples);
        record.lock().unwrap().cleanup_failure = cleanup_failure;
        let mut audio = Some(session);
        let cause = ControllerError::MissingDeviceNode {
            target: crate::RealizationId::LINUX_UHID_USB,
            path: "synthetic-node".into(),
        };
        let error = crate::creation::finish::<()>(Err(cause.clone()), || {
            crate::creation::close_audio(&mut audio)
        })
        .unwrap_err();
        if cleanup_failure {
            match error {
                ControllerError::Cleanup {
                    cause: retained,
                    cleanup,
                } => {
                    assert_eq!(*retained, cause);
                    assert_eq!(cleanup.len(), 1);
                    assert!(cleanup[0].contains("audio cleanup failure"));
                }
                other => panic!("unexpected {other:?}"),
            }
        } else {
            assert_eq!(error, cause);
        }
        drop(audio);
        assert_eq!(record.lock().unwrap().closes, 1);
    }
}

#[test]
fn cleanup_aggregation_keeps_the_first_error_and_every_later_failure() {
    let primary = AudioError::Closed;
    for count in 0..=3 {
        let cleanup = (0..count).map(|index| AudioError::Backend {
            reason: format!("cleanup-{index}"),
        });
        let retained = super::merge_errors(Some(primary.clone()), cleanup).unwrap();
        assert!(retained.to_string().contains(&primary.to_string()));
        for index in 0..count {
            assert!(retained.to_string().contains(&format!("cleanup-{index}")));
        }
    }
    assert_eq!(super::merge_errors(None, []), None);
    assert_eq!(
        super::merge_errors(Some(primary.clone()), []),
        Some(primary)
    );
}

#[test]
fn backend_read_conversion_preserves_application_frame_semantics() {
    use gr_audio_contract::queue::pcm_queue;
    let format = PcmFormat::new(
        48_000,
        &[
            crate::AudioChannel::AudibleLeft,
            crate::AudioChannel::AudibleRight,
        ],
    )
    .unwrap();
    let (mut producer, mut consumer) = pcm_queue(&format, 4).unwrap();
    producer.discard(3).unwrap();
    producer.push(&[101, -202]).unwrap();
    let read = AudioRead::from_backend(consumer.read(&mut [0; 2]).unwrap());
    assert_eq!(
        (read.frames, read.first_frame, read.discontinuity),
        (1, 3, true)
    );
}
