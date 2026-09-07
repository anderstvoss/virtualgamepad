//! Conventional evdev effect policy shared explicitly by the curated controllers.
//! Providers decode the ABI and complete ioctls; this module owns acceptance.
use super::ProviderSessionSink;
use gr_realization_api::{
    ForceFeedbackEffect, ForceFeedbackEvent, ProviderError, ProviderFrame, RawReverseEvent,
    RumbleEffect,
};
use std::collections::VecDeque;

const EFFECTS: usize = 64;
const EVENTS: usize = 32;
const RETRIES: u8 = 8;
const EINVAL: i32 = -22;
const EOPNOTSUPP: i32 = -95;

pub(super) struct Feedback {
    effects: [Option<RumbleEffect>; EFFECTS],
    events: VecDeque<RawReverseEvent>,
    pending: Option<(ProviderFrame, ForceFeedbackEvent)>,
    retries: u8,
}
impl Default for Feedback {
    fn default() -> Self {
        Self {
            effects: [None; EFFECTS],
            events: VecDeque::new(),
            pending: None,
            retries: 0,
        }
    }
}
impl Feedback {
    pub(super) fn pending(&self) -> bool {
        self.pending.is_some() || !self.events.is_empty()
    }
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn service(
        &mut self,
        sink: &mut ProviderSessionSink,
        callback: &mut dyn FnMut(RawReverseEvent),
    ) -> Result<(), ProviderError> {
        let result = self.service_inner(sink, callback);
        if result.is_err() {
            self.clear();
            sink.close();
        }
        result
    }
    fn service_inner(
        &mut self,
        sink: &mut ProviderSessionSink,
        callback: &mut dyn FnMut(RawReverseEvent),
    ) -> Result<(), ProviderError> {
        if !self.flush(sink, callback)? {
            return Ok(());
        }
        if self.events.is_empty() {
            let mut overflow = false;
            sink.drain(&mut |event| {
                if self.events.len() == EVENTS {
                    overflow = true;
                } else {
                    self.events.push_back(event);
                }
            })?;
            if overflow {
                return Err(ProviderError::Read {
                    reason: "evdev reverse batch exceeds bounded ownership".into(),
                });
            }
        }
        while let Some(event) = self.events.pop_front() {
            match event {
                RawReverseEvent::ForceFeedbackUpload { request_id, effect } => {
                    let status = match effect {
                        ForceFeedbackEffect::Rumble(rumble) => {
                            if usize::try_from(rumble.id).map_or(true, |id| id >= EFFECTS) {
                                EINVAL
                            } else if rumble.trigger_button != 0 || rumble.trigger_interval_ms != 0
                            {
                                EOPNOTSUPP
                            } else {
                                0
                            }
                        }
                        ForceFeedbackEffect::Unsupported { .. } => EOPNOTSUPP,
                    };
                    self.pending = Some((
                        ProviderFrame::ForceFeedbackUploadReply { request_id, status },
                        ForceFeedbackEvent::Uploaded {
                            request_id,
                            effect,
                            status,
                        },
                    ));
                }
                RawReverseEvent::ForceFeedbackErase {
                    request_id,
                    effect_id,
                } => {
                    let present = usize::try_from(effect_id)
                        .ok()
                        .and_then(|id| self.effects.get(id))
                        .is_some_and(Option::is_some);
                    let status = if present { 0 } else { EINVAL };
                    self.pending = Some((
                        ProviderFrame::ForceFeedbackEraseReply { request_id, status },
                        ForceFeedbackEvent::Erased {
                            request_id,
                            effect_id,
                            status,
                        },
                    ));
                }
                RawReverseEvent::Evdev(events) => {
                    for event in &events {
                        if event.event_type == super::EV_FF && event.value >= 0 {
                            if let Some(Some(effect)) = self.effects.get(usize::from(event.code)) {
                                callback(RawReverseEvent::ForceFeedback(
                                    ForceFeedbackEvent::Playback {
                                        effect: *effect,
                                        repetitions: u32::try_from(event.value)
                                            .expect("nonnegative count"),
                                    },
                                ));
                            }
                        }
                    }
                    callback(RawReverseEvent::Evdev(events));
                }
                event => callback(event),
            }
            if !self.flush(sink, callback)? {
                return Ok(());
            }
        }
        Ok(())
    }
    fn flush(
        &mut self,
        sink: &mut ProviderSessionSink,
        callback: &mut dyn FnMut(RawReverseEvent),
    ) -> Result<bool, ProviderError> {
        let Some((reply, observation)) = &self.pending else {
            return Ok(true);
        };
        match sink.reply(reply.clone()) {
            Ok(()) => {}
            Err(ProviderError::WouldBlock) if self.retries < RETRIES => {
                self.retries += 1;
                return Ok(false);
            }
            Err(ProviderError::WouldBlock) => {
                return Err(ProviderError::Write {
                    reason: "evdev completion retry budget exhausted".into(),
                });
            }
            Err(error) => return Err(error),
        }
        let observation = *observation;
        match observation {
            ForceFeedbackEvent::Uploaded {
                effect: ForceFeedbackEffect::Rumble(effect),
                status: 0,
                ..
            } => {
                self.effects[usize::try_from(effect.id).expect("validated effect id")] =
                    Some(effect);
            }
            ForceFeedbackEvent::Erased {
                effect_id,
                status: 0,
                ..
            } => {
                self.effects[usize::try_from(effect_id).expect("validated effect id")] = None;
            }
            _ => {}
        }
        self.pending = None;
        self.retries = 0;
        callback(RawReverseEvent::ForceFeedback(observation));
        Ok(true)
    }
}
