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

use std::collections::BTreeMap;

use gr_backend_api::{
    BackendFrame, BackendReverseEvent, BackendReversePayload, BackendReverseTarget, EvdevEvent,
};
use gr_core::{
    BackendLevel, FidelityTier, ProfileId, ProfileInputFrame, ProfileInputPayload,
    SemanticOutputFunction,
};
use gr_profiles::{ProfileFamily, registry};
use gr_runtime_model::{
    AudioCommand, ControllerOutputCommand, LightingPayload, OutputCommandType, OutputFunctionRef,
    OutputPayload, PreparedTranslationContext, RumblePayload, SessionPlan, TranslationConstants,
    TranslatorFamily, TriggerEffectPayload,
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
        family: TranslatorFamily,
        level: BackendLevel,
    ) -> Option<&'static dyn ForwardTranslator> {
        match (family, level) {
            (TranslatorFamily::GenericGamepad, BackendLevel::Evdev) => Some(&GENERIC_EVDEV),
            (TranslatorFamily::XboxStyle, BackendLevel::Evdev) => Some(&XBOX_STYLE_EVDEV),
            (TranslatorFamily::DualSense, BackendLevel::Hid) => Some(&DUALSENSE_USB_HID),
            (TranslatorFamily::SteamController, BackendLevel::Hid) => Some(&STEAM_CONTROLLER_HID),
            _ => None,
        }
    }

    /// Resolve the reverse translator for `family`. Returns `None` if
    /// no translator is registered.
    #[must_use]
    pub fn reverse(&self, family: TranslatorFamily) -> Option<&'static dyn ReverseTranslator> {
        match family {
            TranslatorFamily::XboxStyle => Some(&XBOX_STYLE_REVERSE),
            TranslatorFamily::DualSense => Some(&DUALSENSE_HID_REVERSE),
            TranslatorFamily::SteamController => Some(&STEAM_CONTROLLER_REVERSE),
            _ => None,
        }
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
    plan: &SessionPlan,
    translator_registry: &TranslatorRegistry,
) -> Result<PreparedTranslationContext, TranslationError> {
    let profile = registry().profile(plan.profile_id.clone()).ok_or_else(|| {
        TranslationError::InvalidInput {
            reason: format!("unknown built-in profile `{}`", plan.profile_id),
        }
    })?;

    let forward = translator_registry.forward(plan.selected_translator_family, plan.selected_level);
    if forward.is_none() {
        return Err(TranslationError::NoTranslatorRegistered {
            family: plan.selected_translator_family,
            level: plan.selected_level,
        });
    }

    if reverse_required(profile.profile_family, plan.requested_fidelity_tier)
        && translator_registry
            .reverse(plan.selected_translator_family)
            .is_none()
    {
        return Err(TranslationError::NoTranslatorRegistered {
            family: plan.selected_translator_family,
            level: plan.selected_level,
        });
    }

    let descriptor_template = profile
        .descriptor_templates
        .iter()
        .find(|template| template.fidelity == plan.requested_fidelity_tier)
        .ok_or(TranslationError::DescriptorUnavailable)?;

    Ok(PreparedTranslationContext {
        session_id: Some(plan.session_id),
        profile_family: Some(profile_family_name(profile.profile_family).to_string()),
        host_platform: Some(plan.target_host_platform),
        backend_family: Some(plan.selected_backend_family),
        provider_id: Some(plan.selected_provider_id.clone()),
        level: Some(plan.selected_level),
        session_options: Some(plan.session_options.clone()),
        descriptor_template: Some(descriptor_template),
        translation_constants: TranslationConstants::default(),
    })
}

fn reverse_required(profile_family: ProfileFamily, fidelity: FidelityTier) -> bool {
    !matches!(fidelity, FidelityTier::Compatibility)
        && registry()
            .profiles()
            .iter()
            .find(|profile| profile.profile_family == profile_family)
            .is_some_and(|profile| !profile.reverse_command_support.supported.is_empty())
}

const fn profile_family_name(family: ProfileFamily) -> &'static str {
    match family {
        ProfileFamily::GenericGamepad => "generic-gamepad",
        ProfileFamily::Xbox360 => "xbox360",
        ProfileFamily::DualSense => "dualsense",
        ProfileFamily::SteamController => "steam-controller",
        _ => "unknown",
    }
}

#[derive(Debug)]
struct GenericEvdevTranslator;

impl ForwardTranslator for GenericEvdevTranslator {
    fn family(&self) -> TranslatorFamily {
        TranslatorFamily::GenericGamepad
    }

    fn translate(
        &self,
        input: &ProfileInputFrame,
        _ctx: &PreparedTranslationContext,
        _out: &mut TranslationScratch,
    ) -> Result<BackendFrame, TranslationError> {
        let ProfileInputPayload::GenericGamepad(payload) = &input.payload else {
            return Err(TranslationError::InvalidInput {
                reason: format!(
                    "generic evdev translator expected generic-gamepad payload, got `{}`",
                    input.payload.variant_name()
                ),
            });
        };

        Ok(BackendFrame::EvdevEvents {
            events: generic_evdev_events(payload),
        })
    }
}

#[derive(Debug)]
struct XboxStyleEvdevTranslator;

impl ForwardTranslator for XboxStyleEvdevTranslator {
    fn family(&self) -> TranslatorFamily {
        TranslatorFamily::XboxStyle
    }

    fn translate(
        &self,
        input: &ProfileInputFrame,
        _ctx: &PreparedTranslationContext,
        _out: &mut TranslationScratch,
    ) -> Result<BackendFrame, TranslationError> {
        let ProfileInputPayload::Xbox360(payload) = &input.payload else {
            return Err(TranslationError::InvalidInput {
                reason: format!(
                    "xbox-style evdev translator expected xbox360 payload, got `{}`",
                    input.payload.variant_name()
                ),
            });
        };

        let mut events = Vec::with_capacity(17);
        push_button_event(&mut events, BTN_SOUTH, payload.buttons.face.a);
        push_button_event(&mut events, BTN_EAST, payload.buttons.face.b);
        push_button_event(&mut events, BTN_WEST, payload.buttons.face.x);
        push_button_event(&mut events, BTN_NORTH, payload.buttons.face.y);
        push_button_event(&mut events, BTN_TL, payload.buttons.shoulders.lb);
        push_button_event(&mut events, BTN_TR, payload.buttons.shoulders.rb);
        push_button_event(&mut events, BTN_THUMBL, payload.buttons.stick_clicks.ls);
        push_button_event(&mut events, BTN_THUMBR, payload.buttons.stick_clicks.rs);
        push_button_event(&mut events, BTN_START, payload.buttons.system.start);
        push_button_event(&mut events, BTN_SELECT, payload.buttons.system.back);
        push_button_event(&mut events, BTN_MODE, payload.buttons.system.guide);
        push_axis_event(
            &mut events,
            ABS_HAT0X,
            dpad_axis(payload.dpad.left, payload.dpad.right),
        );
        push_axis_event(
            &mut events,
            ABS_HAT0Y,
            dpad_axis(payload.dpad.up, payload.dpad.down),
        );
        push_axis_event(&mut events, ABS_X, i32::from(payload.sticks.left_x));
        push_axis_event(&mut events, ABS_Y, i32::from(payload.sticks.left_y));
        push_axis_event(&mut events, ABS_RX, i32::from(payload.sticks.right_x));
        push_axis_event(&mut events, ABS_RY, i32::from(payload.sticks.right_y));
        push_axis_event(&mut events, ABS_Z, i32::from(payload.triggers.lt));
        push_axis_event(&mut events, ABS_RZ, i32::from(payload.triggers.rt));

        Ok(BackendFrame::EvdevEvents { events })
    }
}

#[derive(Debug)]
struct DualSenseUsbHidTranslator;

impl ForwardTranslator for DualSenseUsbHidTranslator {
    fn family(&self) -> TranslatorFamily {
        TranslatorFamily::DualSense
    }

    fn translate(
        &self,
        input: &ProfileInputFrame,
        _ctx: &PreparedTranslationContext,
        out: &mut TranslationScratch,
    ) -> Result<BackendFrame, TranslationError> {
        let ProfileInputPayload::DualSense(payload) = &input.payload else {
            return Err(TranslationError::InvalidInput {
                reason: format!(
                    "DualSense HID translator expected dualsense payload, got `{}`",
                    input.payload.variant_name()
                ),
            });
        };

        out.clear();
        out.bytes.resize(DUALSENSE_INPUT_REPORT_LEN, 0);
        out.bytes[0] = encode_axis_u8(payload.sticks.left_x);
        out.bytes[1] = encode_axis_u8(payload.sticks.left_y);
        out.bytes[2] = encode_axis_u8(payload.sticks.right_x);
        out.bytes[3] = encode_axis_u8(payload.sticks.right_y);
        out.bytes[4] = encode_trigger_u8(payload.triggers.l2);
        out.bytes[5] = encode_trigger_u8(payload.triggers.r2);
        out.bytes[7] = encode_dpad_hat(payload.dpad)
            | bool_bit(payload.buttons.face.square, 4)
            | bool_bit(payload.buttons.face.cross, 5)
            | bool_bit(payload.buttons.face.circle, 6)
            | bool_bit(payload.buttons.face.triangle, 7);
        out.bytes[8] = bool_bit(payload.buttons.shoulders.l1, 0)
            | bool_bit(payload.buttons.shoulders.r1, 1)
            | bool_bit(payload.buttons.stick_clicks.l3, 6)
            | bool_bit(payload.buttons.stick_clicks.r3, 7)
            | bool_bit(payload.buttons.system.create, 4)
            | bool_bit(payload.buttons.system.options, 5);
        out.bytes[9] = bool_bit(payload.buttons.system.ps, 0)
            | bool_bit(payload.buttons.system.touchpad_click, 1);
        encode_dualsense_touch_contact(&mut out.bytes[32..36], payload.touchpad.contact_1, 0);
        encode_dualsense_touch_contact(&mut out.bytes[36..40], payload.touchpad.contact_2, 1);
        ensure_hid_report_shape("dualsense", &out.bytes, DUALSENSE_INPUT_REPORT_LEN)?;

        Ok(BackendFrame::HidInputReport {
            report_id: Some(DUALSENSE_INPUT_REPORT_ID),
            bytes: out.bytes.clone(),
        })
    }
}

#[derive(Debug)]
struct SteamControllerHidTranslator;

impl ForwardTranslator for SteamControllerHidTranslator {
    fn family(&self) -> TranslatorFamily {
        TranslatorFamily::SteamController
    }

    fn translate(
        &self,
        input: &ProfileInputFrame,
        _ctx: &PreparedTranslationContext,
        out: &mut TranslationScratch,
    ) -> Result<BackendFrame, TranslationError> {
        let ProfileInputPayload::SteamController(payload) = &input.payload else {
            return Err(TranslationError::InvalidInput {
                reason: format!(
                    "Steam Controller HID translator expected steam-controller payload, got `{}`",
                    input.payload.variant_name()
                ),
            });
        };

        out.clear();
        out.bytes.resize(STEAM_CONTROLLER_INPUT_REPORT_LEN, 0);
        out.bytes[0] = bool_bit(payload.buttons.a, 0)
            | bool_bit(payload.buttons.b, 1)
            | bool_bit(payload.buttons.x, 2)
            | bool_bit(payload.buttons.y, 3)
            | bool_bit(payload.buttons.left_grip, 4)
            | bool_bit(payload.buttons.right_grip, 5)
            | bool_bit(payload.buttons.lb, 6)
            | bool_bit(payload.buttons.rb, 7);
        out.bytes[1] = bool_bit(payload.buttons.menu_primary, 0)
            | bool_bit(payload.buttons.menu_secondary, 1)
            | bool_bit(payload.buttons.steam, 2)
            | bool_bit(payload.buttons.left_pad_click, 3)
            | bool_bit(payload.buttons.right_pad_click, 4)
            | bool_bit(payload.buttons.left_stick_click, 5);
        let mut cursor = 2;
        for value in [
            payload.sticks.left_pad_x,
            payload.sticks.left_pad_y,
            payload.sticks.right_pad_x,
            payload.sticks.right_pad_y,
            payload.sticks.left_stick_x,
            payload.sticks.left_stick_y,
        ] {
            out.bytes[cursor..cursor + 2].copy_from_slice(&value.to_le_bytes());
            cursor += 2;
        }
        out.bytes[cursor..cursor + 2].copy_from_slice(&payload.triggers.lt.to_le_bytes());
        cursor += 2;
        out.bytes[cursor..cursor + 2].copy_from_slice(&payload.triggers.rt.to_le_bytes());
        ensure_hid_report_shape(
            "steam-controller",
            &out.bytes,
            STEAM_CONTROLLER_INPUT_REPORT_LEN,
        )?;

        Ok(BackendFrame::HidInputReport {
            report_id: Some(STEAM_CONTROLLER_INPUT_REPORT_ID),
            bytes: out.bytes.clone(),
        })
    }
}

#[derive(Debug)]
struct XboxStyleReverseTranslator;

impl ReverseTranslator for XboxStyleReverseTranslator {
    fn family(&self) -> TranslatorFamily {
        TranslatorFamily::XboxStyle
    }

    fn translate_reverse(
        &self,
        event: &BackendReverseEvent,
        ctx: &PreparedTranslationContext,
        out: &mut SmallVec<[ControllerOutputCommand; 4]>,
    ) -> Result<(), TranslationError> {
        let bytes = match &event.payload {
            BackendReversePayload::Hid { bytes, .. } => bytes.as_slice(),
            BackendReversePayload::Evdev { events } => {
                for event_item in events {
                    if event_item.code == 0 {
                        out.push(output_command(
                            event,
                            event_profile_id(event, ctx),
                            SemanticOutputFunction::Rumble,
                            OutputPayload::Rumble(RumblePayload {
                                strong: u16::try_from(
                                    event_item.value.clamp(0, i32::from(u16::MAX)),
                                )
                                .expect("clamped rumble value should fit into u16"),
                                weak: u16::try_from(event_item.value.clamp(0, i32::from(u16::MAX)))
                                    .expect("clamped rumble value should fit into u16"),
                            }),
                        ));
                        return Ok(());
                    }
                }
                return Err(TranslationError::InvalidReverseEvent {
                    reason: "xbox-style evdev reverse event did not contain a supported output"
                        .to_string(),
                });
            }
            _ => {
                return Err(TranslationError::InvalidReverseEvent {
                    reason: "xbox-style reverse translator requires HID or evdev payload"
                        .to_string(),
                });
            }
        };
        let profile_id = event_profile_id(event, ctx);
        if event_target_is(event, SemanticOutputFunction::Lighting) {
            if bytes.len() >= 3 {
                out.push(output_command(
                    event,
                    profile_id,
                    SemanticOutputFunction::Lighting,
                    OutputPayload::Lighting(LightingPayload {
                        red: None,
                        green: None,
                        blue: None,
                        player_index: Some(bytes[0] & 0x0f),
                    }),
                ));
                return Ok(());
            }
        } else if event_target_is(event, SemanticOutputFunction::PlayerIndicators) {
            if !bytes.is_empty() {
                out.push(output_command(
                    event,
                    profile_id,
                    SemanticOutputFunction::PlayerIndicators,
                    OutputPayload::Lighting(LightingPayload {
                        red: None,
                        green: None,
                        blue: None,
                        player_index: Some(bytes[0] & 0x0f),
                    }),
                ));
                return Ok(());
            }
        } else if event_target_is_or_unspecified(event, SemanticOutputFunction::Rumble)
            && bytes.len() >= 2
        {
            out.push(output_command(
                event,
                profile_id,
                SemanticOutputFunction::Rumble,
                OutputPayload::Rumble(RumblePayload {
                    strong: scale_u8_to_u16(bytes[0]),
                    weak: scale_u8_to_u16(bytes[1]),
                }),
            ));
            return Ok(());
        }
        Err(TranslationError::InvalidReverseEvent {
            reason: "xbox-style reverse event did not contain any recognized output command"
                .to_string(),
        })
    }
}

#[derive(Debug)]
struct DualSenseHidReverseTranslator;

impl ReverseTranslator for DualSenseHidReverseTranslator {
    fn family(&self) -> TranslatorFamily {
        TranslatorFamily::DualSense
    }

    fn translate_reverse(
        &self,
        event: &BackendReverseEvent,
        ctx: &PreparedTranslationContext,
        out: &mut SmallVec<[ControllerOutputCommand; 4]>,
    ) -> Result<(), TranslationError> {
        let BackendReversePayload::Hid { bytes, .. } = &event.payload else {
            return Err(TranslationError::InvalidReverseEvent {
                reason: "DualSense reverse translator requires HID payload".to_string(),
            });
        };
        let profile_id = event_profile_id(event, ctx);

        for command in [
            decode_dualsense_rumble(event, &profile_id, bytes),
            decode_dualsense_lighting(event, &profile_id, bytes),
            decode_dualsense_player_indicators(event, &profile_id, bytes),
            decode_dualsense_trigger_effect(event, &profile_id, bytes),
            decode_dualsense_audio(event, &profile_id, bytes),
        ]
        .into_iter()
        .flatten()
        {
            out.push(command);
        }

        if out.is_empty() {
            return Err(TranslationError::InvalidReverseEvent {
                reason: "DualSense reverse event did not contain any recognized output command"
                    .to_string(),
            });
        }
        Ok(())
    }
}

fn decode_dualsense_rumble(
    event: &BackendReverseEvent,
    profile_id: &ProfileId,
    bytes: &[u8],
) -> Option<ControllerOutputCommand> {
    // Rumble and Haptics share the same output byte positions on
    // DualSense; either explicit target accepts the decode, and an
    // unspecified target falls through.
    let target = event.target.as_ref();
    let accepts = matches!(
        target,
        Some(BackendReverseTarget::SemanticOutput(
            SemanticOutputFunction::Rumble | SemanticOutputFunction::Haptics
        ))
    ) || target.is_none();
    if !accepts || bytes.len() <= 4 || (bytes[3] == 0 && bytes[4] == 0) {
        return None;
    }
    Some(output_command(
        event,
        profile_id.clone(),
        SemanticOutputFunction::Rumble,
        OutputPayload::Rumble(RumblePayload {
            strong: scale_u8_to_u16(bytes[4]),
            weak: scale_u8_to_u16(bytes[3]),
        }),
    ))
}

fn decode_dualsense_lighting(
    event: &BackendReverseEvent,
    profile_id: &ProfileId,
    bytes: &[u8],
) -> Option<ControllerOutputCommand> {
    if !event_target_is_or_unspecified(event, SemanticOutputFunction::Lighting)
        || bytes.len() <= 47
        || (bytes[45] == 0 && bytes[46] == 0 && bytes[47] == 0)
    {
        return None;
    }
    Some(output_command(
        event,
        profile_id.clone(),
        SemanticOutputFunction::Lighting,
        OutputPayload::Lighting(LightingPayload {
            red: Some(bytes[45]),
            green: Some(bytes[46]),
            blue: Some(bytes[47]),
            player_index: None,
        }),
    ))
}

fn decode_dualsense_player_indicators(
    event: &BackendReverseEvent,
    profile_id: &ProfileId,
    bytes: &[u8],
) -> Option<ControllerOutputCommand> {
    if !event_target_is_or_unspecified(event, SemanticOutputFunction::PlayerIndicators)
        || bytes.len() <= 44
        || bytes[44] == 0
    {
        return None;
    }
    Some(output_command(
        event,
        profile_id.clone(),
        SemanticOutputFunction::PlayerIndicators,
        OutputPayload::Lighting(LightingPayload {
            red: None,
            green: None,
            blue: None,
            player_index: Some(bytes[44] & 0x0f),
        }),
    ))
}

fn decode_dualsense_trigger_effect(
    event: &BackendReverseEvent,
    profile_id: &ProfileId,
    bytes: &[u8],
) -> Option<ControllerOutputCommand> {
    if !event_target_is_or_unspecified(event, SemanticOutputFunction::TriggerEffect)
        || bytes.len() <= 13
        || bytes[11] == 0
    {
        return None;
    }
    let mut parameters = BTreeMap::new();
    parameters.insert("left-strength".to_string(), format!("{}", bytes[12]));
    parameters.insert("right-strength".to_string(), format!("{}", bytes[13]));
    Some(output_command(
        event,
        profile_id.clone(),
        SemanticOutputFunction::TriggerEffect,
        OutputPayload::TriggerEffect(TriggerEffectPayload {
            mode: format!("0x{:02x}", bytes[11]),
            parameters,
        }),
    ))
}

fn decode_dualsense_audio(
    event: &BackendReverseEvent,
    profile_id: &ProfileId,
    bytes: &[u8],
) -> Option<ControllerOutputCommand> {
    if !event_target_is_or_unspecified(event, SemanticOutputFunction::Audio)
        || bytes.len() <= 9
        || bytes[9] == 0
    {
        return None;
    }
    Some(output_command(
        event,
        profile_id.clone(),
        SemanticOutputFunction::Audio,
        OutputPayload::Audio(AudioCommand {
            action: "speaker-update".to_string(),
            target: Some("speaker".to_string()),
        }),
    ))
}

#[derive(Debug)]
struct SteamControllerReverseTranslator;

impl ReverseTranslator for SteamControllerReverseTranslator {
    fn family(&self) -> TranslatorFamily {
        TranslatorFamily::SteamController
    }

    fn translate_reverse(
        &self,
        event: &BackendReverseEvent,
        ctx: &PreparedTranslationContext,
        out: &mut SmallVec<[ControllerOutputCommand; 4]>,
    ) -> Result<(), TranslationError> {
        let BackendReversePayload::Hid { bytes, .. } = &event.payload else {
            return Err(TranslationError::InvalidReverseEvent {
                reason: "Steam Controller reverse translator requires HID payload".to_string(),
            });
        };
        let profile_id = event_profile_id(event, ctx);

        if event_target_is_or_unspecified(event, SemanticOutputFunction::Lighting)
            && bytes.len() >= 4
        {
            out.push(output_command(
                event,
                profile_id.clone(),
                SemanticOutputFunction::Lighting,
                OutputPayload::Lighting(LightingPayload {
                    red: Some(bytes[0]),
                    green: Some(bytes[1]),
                    blue: Some(bytes[2]),
                    player_index: None,
                }),
            ));
        }
        if event_target_is_or_unspecified(event, SemanticOutputFunction::Rumble)
            && bytes.len() >= 6
            && (bytes[4] != 0 || bytes[5] != 0)
        {
            out.push(output_command(
                event,
                profile_id,
                SemanticOutputFunction::Rumble,
                OutputPayload::Rumble(RumblePayload {
                    strong: scale_u8_to_u16(bytes[5]),
                    weak: scale_u8_to_u16(bytes[4]),
                }),
            ));
        }

        if out.is_empty() {
            return Err(TranslationError::InvalidReverseEvent {
                reason:
                    "Steam Controller reverse event did not contain any recognized output command"
                        .to_string(),
            });
        }
        Ok(())
    }
}

const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const BTN_SOUTH: u16 = 0x130;
const BTN_EAST: u16 = 0x131;
const BTN_NORTH: u16 = 0x133;
const BTN_WEST: u16 = 0x134;
const BTN_TL: u16 = 0x136;
const BTN_TR: u16 = 0x137;
const BTN_SELECT: u16 = 0x13a;
const BTN_START: u16 = 0x13b;
const BTN_MODE: u16 = 0x13c;
const BTN_THUMBL: u16 = 0x13d;
const BTN_THUMBR: u16 = 0x13e;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_Z: u16 = 0x02;
const ABS_RX: u16 = 0x03;
const ABS_RY: u16 = 0x04;
const ABS_RZ: u16 = 0x05;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;
const DUALSENSE_INPUT_REPORT_ID: u8 = 0x01;
const DUALSENSE_INPUT_REPORT_LEN: usize = 64;
const STEAM_CONTROLLER_INPUT_REPORT_ID: u8 = 0x01;
const STEAM_CONTROLLER_INPUT_REPORT_LEN: usize = 18;

fn generic_evdev_events(payload: &gr_core::GenericGamepadInput) -> Vec<EvdevEvent> {
    let mut events = Vec::with_capacity(19);
    push_button_event(&mut events, BTN_SOUTH, payload.buttons.south);
    push_button_event(&mut events, BTN_EAST, payload.buttons.east);
    push_button_event(&mut events, BTN_WEST, payload.buttons.west);
    push_button_event(&mut events, BTN_NORTH, payload.buttons.north);
    push_button_event(&mut events, BTN_TL, payload.buttons.left_shoulder);
    push_button_event(&mut events, BTN_TR, payload.buttons.right_shoulder);
    push_button_event(&mut events, BTN_THUMBL, payload.buttons.left_stick_button);
    push_button_event(&mut events, BTN_THUMBR, payload.buttons.right_stick_button);
    push_button_event(&mut events, BTN_START, payload.buttons.menu_primary);
    push_button_event(&mut events, BTN_SELECT, payload.buttons.menu_secondary);
    push_button_event(&mut events, BTN_MODE, payload.buttons.guide);
    push_axis_event(
        &mut events,
        ABS_HAT0X,
        dpad_axis(payload.dpad.left, payload.dpad.right),
    );
    push_axis_event(
        &mut events,
        ABS_HAT0Y,
        dpad_axis(payload.dpad.up, payload.dpad.down),
    );
    push_axis_event(&mut events, ABS_X, i32::from(payload.sticks.left_x));
    push_axis_event(&mut events, ABS_Y, i32::from(payload.sticks.left_y));
    push_axis_event(&mut events, ABS_RX, i32::from(payload.sticks.right_x));
    push_axis_event(&mut events, ABS_RY, i32::from(payload.sticks.right_y));
    push_axis_event(&mut events, ABS_Z, i32::from(payload.triggers.left_trigger));
    push_axis_event(
        &mut events,
        ABS_RZ,
        i32::from(payload.triggers.right_trigger),
    );
    events
}

fn push_button_event(events: &mut Vec<EvdevEvent>, code: u16, pressed: bool) {
    events.push(EvdevEvent {
        event_type: EV_KEY,
        code,
        value: i32::from(pressed),
    });
}

fn push_axis_event(events: &mut Vec<EvdevEvent>, code: u16, value: i32) {
    events.push(EvdevEvent {
        event_type: EV_ABS,
        code,
        value,
    });
}

fn dpad_axis(negative: bool, positive: bool) -> i32 {
    match (negative, positive) {
        (true, false) => -1,
        (false, true) => 1,
        _ => 0,
    }
}

fn encode_axis_u8(value: i16) -> u8 {
    let scaled = i32::from(value) + 32_768;
    u8::try_from((scaled * 255) / 65_535).expect("scaled axis should fit into u8")
}

fn encode_trigger_u8(value: u16) -> u8 {
    (value / 257) as u8
}

fn scale_u8_to_u16(value: u8) -> u16 {
    u16::from(value) * 257
}

fn bool_bit(enabled: bool, bit: u8) -> u8 {
    if enabled { 1 << bit } else { 0 }
}

fn encode_dpad_hat(dpad: gr_core::Dpad) -> u8 {
    match (dpad.up, dpad.right, dpad.down, dpad.left) {
        (true, false, false, false) => 0x00,
        (true, true, false, false) => 0x01,
        (false, true, false, false) => 0x02,
        (false, true, true, false) => 0x03,
        (false, false, true, false) => 0x04,
        (false, false, true, true) => 0x05,
        (false, false, false, true) => 0x06,
        (true, false, false, true) => 0x07,
        _ => 0x08,
    }
}

fn encode_dualsense_touch_contact(
    bytes: &mut [u8],
    contact: gr_core::DualSenseTouchContact,
    counter: u8,
) {
    let x = contact.x.min(0x0fff);
    let y = contact.y.min(0x0fff);
    bytes[0] = if contact.active {
        counter & 0x7f
    } else {
        0x80 | (counter & 0x7f)
    };
    bytes[1] = (x & 0xff) as u8;
    bytes[2] = ((x >> 8) as u8 & 0x0f) | (((y & 0x0f) as u8) << 4);
    bytes[3] = (y >> 4) as u8;
}

fn ensure_hid_report_shape(
    profile_family: &str,
    bytes: &[u8],
    expected_len: usize,
) -> Result<(), TranslationError> {
    if bytes.len() != expected_len {
        return Err(TranslationError::DescriptorViolation {
            reason: format!(
                "{profile_family} translator emitted {} bytes, expected {expected_len}",
                bytes.len()
            ),
        });
    }
    Ok(())
}

fn event_profile_id(event: &BackendReverseEvent, ctx: &PreparedTranslationContext) -> ProfileId {
    event.profile_id.clone().unwrap_or_else(|| {
        ProfileId::from(ctx.profile_family.as_deref().unwrap_or("unknown-profile"))
    })
}

/// Returns true if the event's `target` names exactly `expected` as a
/// `SemanticOutput`. Used by reverse translators to gate per-output
/// decode paths when the host has narrowed the target.
fn event_target_is(event: &BackendReverseEvent, expected: SemanticOutputFunction) -> bool {
    matches!(
        event.target.as_ref(),
        Some(BackendReverseTarget::SemanticOutput(function)) if *function == expected
    )
}

/// Returns true if the event's `target` is `None` (host did not narrow)
/// or if it names exactly `expected` as a `SemanticOutput`. Used by
/// reverse translators that decode multiple output sections from one
/// event payload and should fall through when the target is broad.
fn event_target_is_or_unspecified(
    event: &BackendReverseEvent,
    expected: SemanticOutputFunction,
) -> bool {
    event.target.is_none() || event_target_is(event, expected)
}

fn output_command(
    event: &BackendReverseEvent,
    profile_id: ProfileId,
    function: SemanticOutputFunction,
    payload: OutputPayload,
) -> ControllerOutputCommand {
    ControllerOutputCommand {
        session_id: event.session_id,
        profile_id,
        timestamp: event.timestamp,
        command_type: OutputCommandType::StateUpdate,
        function: OutputFunctionRef::Semantic(function),
        payload,
    }
}

static GENERIC_EVDEV: GenericEvdevTranslator = GenericEvdevTranslator;
static XBOX_STYLE_EVDEV: XboxStyleEvdevTranslator = XboxStyleEvdevTranslator;
static DUALSENSE_USB_HID: DualSenseUsbHidTranslator = DualSenseUsbHidTranslator;
static STEAM_CONTROLLER_HID: SteamControllerHidTranslator = SteamControllerHidTranslator;
static XBOX_STYLE_REVERSE: XboxStyleReverseTranslator = XboxStyleReverseTranslator;
static DUALSENSE_HID_REVERSE: DualSenseHidReverseTranslator = DualSenseHidReverseTranslator;
static STEAM_CONTROLLER_REVERSE: SteamControllerReverseTranslator =
    SteamControllerReverseTranslator;

#[cfg(test)]
mod tests {
    use super::{
        DUALSENSE_INPUT_REPORT_ID, STEAM_CONTROLLER_INPUT_REPORT_ID, TranslationError,
        TranslationScratch, TranslatorRegistry, prepared_translation_context,
    };
    use gr_backend_api::{BackendReverseEvent, BackendReversePayload, BackendReverseTarget};
    use gr_core::{
        BackendFamily, BackendLevel, FidelityTier, ProfileId, ProfileInputFrame,
        ProfileInputPayload, SemanticOutputFunction, SessionId, Timestamp,
    };
    use gr_runtime_model::{
        BackendOpenContext, BackpressurePolicy, CapabilityNegotiationResult, DegradationReport,
        DeploymentRequirements, EmulationGoal, HostPlatform, OutputPayload,
        ReverseEventDeliveryPolicy, SessionOptionsSnapshot, SessionPlan, TranslatorFamily,
    };
    use proptest::prelude::*;
    use smallvec::SmallVec;

    fn base_plan() -> SessionPlan {
        SessionPlan {
            session_id: SessionId::new(7),
            profile_id: ProfileId::from("dualsense"),
            requested_goal: EmulationGoal::IdentityAware,
            requested_fidelity_tier: FidelityTier::IdentityAware,
            selected_level: BackendLevel::Hid,
            target_host_platform: HostPlatform::Linux,
            selected_backend_family: BackendFamily::LinuxUhid,
            selected_provider_id: "linux-uhid".into(),
            selected_translator_family: TranslatorFamily::DualSense,
            capability_result: CapabilityNegotiationResult::default(),
            degradation: DegradationReport::default(),
            warnings: Vec::new(),
            deployment_requirements: DeploymentRequirements::default(),
            backend_open_context: BackendOpenContext {
                session_id: SessionId::new(7),
                profile_id: ProfileId::from("dualsense"),
                fidelity_tier: FidelityTier::IdentityAware,
                backend_level: BackendLevel::Hid,
                host_platform: HostPlatform::Linux,
            },
            session_options: SessionOptionsSnapshot {
                accepted_update_kinds: vec!["frame".to_string()],
                unknown_field_policy: "reject".to_string(),
                range_validation_policy: "reject".to_string(),
                coerce_integer_like_values: false,
                allow_missing_optional_fields: true,
                require_monotonic_sequence: false,
                preferred_provider: Some("linux-uhid".into()),
                reject_unsupported_provider_preference: true,
                unsupported_capability_policy: "report".to_string(),
                delivery_policy: ReverseEventDeliveryPolicy::Callback {
                    callback_namespace: "virtualGamepad".to_string(),
                },
                backpressure_policy: BackpressurePolicy::DropOldest {
                    log_dropped_outputs: true,
                    max_queue_depth: Some(8),
                },
            },
        }
    }

    #[test]
    fn registry_resolves_known_phase6_entries() {
        let registry = TranslatorRegistry::new();
        assert!(
            registry
                .forward(TranslatorFamily::DualSense, BackendLevel::Hid)
                .is_some()
        );
        assert!(registry.reverse(TranslatorFamily::DualSense).is_some());
    }

    #[test]
    fn prepared_context_populates_live_descriptor_template() {
        let registry = TranslatorRegistry::new();
        let plan = base_plan();

        let ctx = prepared_translation_context(&plan, &registry).expect("context");
        assert_eq!(ctx.session_id, Some(SessionId::new(7)));
        assert_eq!(ctx.profile_family.as_deref(), Some("dualsense"));
        assert_eq!(ctx.level, Some(BackendLevel::Hid));
        assert!(ctx.descriptor_template.is_some());
    }

    #[test]
    fn prepared_context_reports_missing_translator() {
        let registry = TranslatorRegistry::new();
        let mut plan = base_plan();
        plan.selected_level = BackendLevel::Transport;

        let error = prepared_translation_context(&plan, &registry).expect_err("missing");
        assert_eq!(
            error,
            TranslationError::NoTranslatorRegistered {
                family: TranslatorFamily::DualSense,
                level: BackendLevel::Transport
            }
        );
    }

    #[test]
    fn prepared_context_reports_missing_descriptor_for_selected_tier() {
        let registry = TranslatorRegistry::new();
        let mut plan = base_plan();
        plan.profile_id = ProfileId::from("generic-gamepad");
        plan.requested_goal = EmulationGoal::Compatibility;
        plan.requested_fidelity_tier = FidelityTier::IdentityAware;
        plan.selected_level = BackendLevel::Evdev;
        plan.selected_backend_family = BackendFamily::LinuxUinput;
        plan.selected_provider_id = "linux-uinput".into();
        plan.selected_translator_family = TranslatorFamily::GenericGamepad;
        plan.backend_open_context = BackendOpenContext {
            session_id: SessionId::new(7),
            profile_id: ProfileId::from("generic-gamepad"),
            fidelity_tier: FidelityTier::IdentityAware,
            backend_level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
        };

        let error = prepared_translation_context(&plan, &registry).expect_err("missing");
        assert_eq!(error, TranslationError::DescriptorUnavailable);
    }

    #[test]
    fn prepared_context_supports_evdev_translator_family_resolution() {
        let registry = TranslatorRegistry::new();
        let mut plan = base_plan();
        plan.profile_id = ProfileId::from("xbox360");
        plan.requested_goal = EmulationGoal::Compatibility;
        plan.requested_fidelity_tier = FidelityTier::Compatibility;
        plan.selected_level = BackendLevel::Evdev;
        plan.selected_backend_family = BackendFamily::LinuxUinput;
        plan.selected_provider_id = "linux-uinput".into();
        plan.selected_translator_family = TranslatorFamily::XboxStyle;
        plan.backend_open_context = BackendOpenContext {
            session_id: SessionId::new(7),
            profile_id: ProfileId::from("xbox360"),
            fidelity_tier: FidelityTier::Compatibility,
            backend_level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
        };

        let ctx = prepared_translation_context(&plan, &registry).expect("context");
        assert_eq!(ctx.profile_family.as_deref(), Some("xbox360"));
        assert_eq!(ctx.level, Some(BackendLevel::Evdev));
    }

    #[test]
    fn xbox_evdev_translation_emits_expected_buttons_and_axes() {
        let registry = TranslatorRegistry::new();
        let mut plan = base_plan();
        plan.profile_id = ProfileId::from("xbox360");
        plan.requested_goal = EmulationGoal::Compatibility;
        plan.requested_fidelity_tier = FidelityTier::Compatibility;
        plan.selected_level = BackendLevel::Evdev;
        plan.selected_backend_family = BackendFamily::LinuxUinput;
        plan.selected_provider_id = "linux-uinput".into();
        plan.selected_translator_family = TranslatorFamily::XboxStyle;
        plan.backend_open_context = BackendOpenContext {
            session_id: SessionId::new(7),
            profile_id: ProfileId::from("xbox360"),
            fidelity_tier: FidelityTier::Compatibility,
            backend_level: BackendLevel::Evdev,
            host_platform: HostPlatform::Linux,
        };
        let ctx = prepared_translation_context(&plan, &registry).expect("context");
        let translator = registry
            .forward(TranslatorFamily::XboxStyle, BackendLevel::Evdev)
            .expect("translator");
        let frame = ProfileInputFrame {
            profile_id: ProfileId::from("xbox360"),
            timestamp: Timestamp::new(1),
            sequence: 1.into(),
            payload: ProfileInputPayload::Xbox360(gr_core::Xbox360Input {
                buttons: gr_core::Xbox360Buttons {
                    face: gr_core::Xbox360FaceButtons {
                        a: true,
                        ..gr_core::Xbox360FaceButtons::neutral()
                    },
                    shoulders: gr_core::Xbox360Shoulders::neutral(),
                    stick_clicks: gr_core::Xbox360StickClicks::neutral(),
                    system: gr_core::Xbox360SystemButtons::neutral(),
                },
                dpad: gr_core::Dpad {
                    right: true,
                    ..gr_core::Dpad::neutral()
                },
                sticks: gr_core::TwinStickAxes {
                    left_x: 1200,
                    left_y: -2400,
                    right_x: 3000,
                    right_y: -4000,
                },
                triggers: gr_core::Xbox360Triggers { lt: 10, rt: 20 },
            }),
        };
        let mut scratch = TranslationScratch::new();
        let translated = translator
            .translate(&frame, &ctx, &mut scratch)
            .expect("translate");
        let gr_backend_api::BackendFrame::EvdevEvents { events } = translated else {
            panic!("expected evdev events");
        };
        assert!(
            events
                .iter()
                .any(|event| event.code == super::BTN_SOUTH && event.value == 1)
        );
        assert!(
            events
                .iter()
                .any(|event| event.code == super::ABS_HAT0X && event.value == 1)
        );
        assert!(
            events
                .iter()
                .any(|event| event.code == super::ABS_X && event.value == 1200)
        );
        assert!(
            events
                .iter()
                .any(|event| event.code == super::ABS_RZ && event.value == 20)
        );
    }

    #[test]
    fn dualsense_hid_translation_encodes_buttons_touch_and_report_id() {
        let registry = TranslatorRegistry::new();
        let plan = base_plan();
        let ctx = prepared_translation_context(&plan, &registry).expect("context");
        let translator = registry
            .forward(TranslatorFamily::DualSense, BackendLevel::Hid)
            .expect("translator");
        let frame = ProfileInputFrame {
            profile_id: ProfileId::from("dualsense"),
            timestamp: Timestamp::new(1),
            sequence: 1.into(),
            payload: ProfileInputPayload::DualSense(gr_core::DualSenseInput {
                buttons: gr_core::DualSenseButtons {
                    face: gr_core::DualSenseFaceButtons {
                        cross: true,
                        triangle: true,
                        ..gr_core::DualSenseFaceButtons::neutral()
                    },
                    shoulders: gr_core::DualSenseShoulders {
                        l1: true,
                        r1: false,
                    },
                    stick_clicks: gr_core::DualSenseStickClicks::neutral(),
                    system: gr_core::DualSenseSystemButtons {
                        touchpad_click: true,
                        ..gr_core::DualSenseSystemButtons::neutral()
                    },
                },
                dpad: gr_core::Dpad {
                    up: true,
                    ..gr_core::Dpad::neutral()
                },
                sticks: gr_core::TwinStickAxes::neutral(),
                triggers: gr_core::DualSenseTriggers { l2: 1028, r2: 2056 },
                touchpad: gr_core::DualSenseTouchpad {
                    contact_1: gr_core::DualSenseTouchContact {
                        active: true,
                        x: 1000,
                        y: 512,
                    },
                    contact_2: gr_core::DualSenseTouchContact::neutral(),
                },
            }),
        };
        let mut scratch = TranslationScratch::new();
        let translated = translator
            .translate(&frame, &ctx, &mut scratch)
            .expect("translate");
        let gr_backend_api::BackendFrame::HidInputReport { report_id, bytes } = translated else {
            panic!("expected hid report");
        };
        assert_eq!(report_id, Some(DUALSENSE_INPUT_REPORT_ID));
        assert_eq!(bytes[7] & 0x0f, 0x00);
        assert_ne!(bytes[7] & 0b1111_0000, 0);
        assert_eq!(bytes[8] & 0x01, 0x01);
        assert_eq!(bytes[9] & 0x02, 0x02);
        assert_eq!(bytes[32] & 0x80, 0x00);
    }

    #[test]
    fn steam_hid_translation_encodes_axes_and_triggers() {
        let registry = TranslatorRegistry::new();
        let mut plan = base_plan();
        plan.profile_id = ProfileId::from("steam-controller");
        plan.selected_translator_family = TranslatorFamily::SteamController;
        let ctx = prepared_translation_context(&plan, &registry).expect("context");
        let translator = registry
            .forward(TranslatorFamily::SteamController, BackendLevel::Hid)
            .expect("translator");
        let frame = ProfileInputFrame {
            profile_id: ProfileId::from("steam-controller"),
            timestamp: Timestamp::new(1),
            sequence: 1.into(),
            payload: ProfileInputPayload::SteamController(gr_core::SteamControllerInput {
                buttons: gr_core::SteamControllerButtons {
                    a: true,
                    steam: true,
                    ..gr_core::SteamControllerButtons::neutral()
                },
                sticks: gr_core::SteamControllerSticks {
                    left_pad_x: 11,
                    left_pad_y: -22,
                    right_pad_x: 33,
                    right_pad_y: -44,
                    left_stick_x: 55,
                    left_stick_y: -66,
                },
                triggers: gr_core::SteamControllerTriggers { lt: 77, rt: 88 },
            }),
        };
        let mut scratch = TranslationScratch::new();
        let translated = translator
            .translate(&frame, &ctx, &mut scratch)
            .expect("translate");
        let gr_backend_api::BackendFrame::HidInputReport { report_id, bytes } = translated else {
            panic!("expected hid report");
        };
        assert_eq!(report_id, Some(STEAM_CONTROLLER_INPUT_REPORT_ID));
        assert_eq!(bytes[0] & 0x01, 0x01);
        assert_eq!(bytes[1] & 0x04, 0x04);
        assert_eq!(i16::from_le_bytes([bytes[2], bytes[3]]), 11);
        assert_eq!(u16::from_le_bytes([bytes[14], bytes[15]]), 77);
        assert_eq!(u16::from_le_bytes([bytes[16], bytes[17]]), 88);
    }

    #[test]
    fn dualsense_reverse_translation_decodes_rumble() {
        let registry = TranslatorRegistry::new();
        let plan = base_plan();
        let ctx = prepared_translation_context(&plan, &registry).expect("context");
        let translator = registry
            .reverse(TranslatorFamily::DualSense)
            .expect("translator");
        let event = BackendReverseEvent {
            session_id: SessionId::new(7),
            profile_id: Some(ProfileId::from("dualsense")),
            timestamp: Timestamp::new(10),
            sequence: 1.into(),
            kind: gr_backend_api::BackendReverseEventKind::HidOutputReport,
            target: Some(BackendReverseTarget::SemanticOutput(
                SemanticOutputFunction::Rumble,
            )),
            payload: BackendReversePayload::Hid {
                report_id: Some(0x02),
                bytes: {
                    let mut bytes = vec![0_u8; 48];
                    bytes[3] = 10;
                    bytes[4] = 20;
                    bytes
                },
            },
        };
        let mut out = SmallVec::<[_; 4]>::new();
        translator
            .translate_reverse(&event, &ctx, &mut out)
            .expect("reverse");
        assert_eq!(out.len(), 1);
        match &out[0].payload {
            OutputPayload::Rumble(payload) => {
                assert_eq!(payload.strong, 20 * 257);
                assert_eq!(payload.weak, 10 * 257);
            }
            other => panic!("expected rumble, got {other:?}"),
        }
    }

    #[test]
    fn steam_reverse_translation_decodes_lighting() {
        let registry = TranslatorRegistry::new();
        let mut plan = base_plan();
        plan.profile_id = ProfileId::from("steam-controller");
        plan.selected_translator_family = TranslatorFamily::SteamController;
        let ctx = prepared_translation_context(&plan, &registry).expect("context");
        let translator = registry
            .reverse(TranslatorFamily::SteamController)
            .expect("translator");
        let event = BackendReverseEvent {
            session_id: SessionId::new(7),
            profile_id: Some(ProfileId::from("steam-controller")),
            timestamp: Timestamp::new(10),
            sequence: 1.into(),
            kind: gr_backend_api::BackendReverseEventKind::HidOutputReport,
            target: Some(BackendReverseTarget::SemanticOutput(
                SemanticOutputFunction::Lighting,
            )),
            payload: BackendReversePayload::Hid {
                report_id: Some(0x02),
                bytes: vec![1, 2, 3, 0, 0, 0],
            },
        };
        let mut out = SmallVec::<[_; 4]>::new();
        translator
            .translate_reverse(&event, &ctx, &mut out)
            .expect("reverse");
        assert_eq!(out.len(), 1);
        match &out[0].payload {
            OutputPayload::Lighting(payload) => {
                assert_eq!(payload.red, Some(1));
                assert_eq!(payload.green, Some(2));
                assert_eq!(payload.blue, Some(3));
            }
            other => panic!("expected lighting, got {other:?}"),
        }
    }

    fn assert_declared_outputs(
        outputs: &[super::ControllerOutputCommand],
        allowed: &[SemanticOutputFunction],
    ) {
        for output in outputs {
            let gr_runtime_model::OutputFunctionRef::Semantic(function) = output.function else {
                panic!("unexpected non-semantic output");
            };
            assert!(
                allowed.contains(&function),
                "emitted undeclared output function {function}"
            );
        }
    }

    /// Read the profile registry to produce the list of semantic
    /// output functions a profile declares as reverse-supported.
    /// Sourcing the allowed list from the registry means the property
    /// test follows the profile contract automatically: if a profile
    /// adds or removes a reverse output, the test reflects it without
    /// a separate edit here.
    fn declared_semantic_outputs(profile_id: &str) -> Vec<SemanticOutputFunction> {
        use gr_profiles::{OutputFunctionRef, registry};
        registry()
            .profile_by_str(profile_id)
            .expect("registered profile")
            .reverse_command_support
            .supported
            .iter()
            .filter_map(|function| match function {
                OutputFunctionRef::Semantic(output) => Some(*output),
                _ => None,
            })
            .collect()
    }

    proptest! {
        #[test]
        fn reverse_translators_never_emit_undeclared_outputs_dualsense(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
            let registry = TranslatorRegistry::new();
            let plan = base_plan();
            let ctx = prepared_translation_context(&plan, &registry).expect("context");
            let translator = registry.reverse(TranslatorFamily::DualSense).expect("translator");
            let event = BackendReverseEvent {
                session_id: SessionId::new(7),
                profile_id: Some(ProfileId::from("dualsense")),
                timestamp: Timestamp::new(1),
                sequence: 1.into(),
                kind: gr_backend_api::BackendReverseEventKind::HidOutputReport,
                target: None,
                payload: BackendReversePayload::Hid { report_id: Some(2), bytes },
            };
            let mut out = SmallVec::<[_; 4]>::new();
            let _ = translator.translate_reverse(&event, &ctx, &mut out);
            assert_declared_outputs(&out, &declared_semantic_outputs("dualsense"));
        }

        #[test]
        fn reverse_translators_never_emit_undeclared_outputs_steam(bytes in proptest::collection::vec(any::<u8>(), 0..16)) {
            let registry = TranslatorRegistry::new();
            let mut plan = base_plan();
            plan.profile_id = ProfileId::from("steam-controller");
            plan.selected_translator_family = TranslatorFamily::SteamController;
            let ctx = prepared_translation_context(&plan, &registry).expect("context");
            let translator = registry.reverse(TranslatorFamily::SteamController).expect("translator");
            let event = BackendReverseEvent {
                session_id: SessionId::new(7),
                profile_id: Some(ProfileId::from("steam-controller")),
                timestamp: Timestamp::new(1),
                sequence: 1.into(),
                kind: gr_backend_api::BackendReverseEventKind::HidOutputReport,
                target: None,
                payload: BackendReversePayload::Hid { report_id: Some(2), bytes },
            };
            let mut out = SmallVec::<[_; 4]>::new();
            let _ = translator.translate_reverse(&event, &ctx, &mut out);
            assert_declared_outputs(&out, &declared_semantic_outputs("steam-controller"));
        }

        #[test]
        fn reverse_translators_never_emit_undeclared_outputs_xbox(bytes in proptest::collection::vec(any::<u8>(), 0..8)) {
            let registry = TranslatorRegistry::new();
            let mut plan = base_plan();
            plan.profile_id = ProfileId::from("xbox360");
            plan.requested_goal = EmulationGoal::Compatibility;
            plan.requested_fidelity_tier = FidelityTier::Compatibility;
            plan.selected_level = BackendLevel::Evdev;
            plan.selected_backend_family = BackendFamily::LinuxUinput;
            plan.selected_provider_id = "linux-uinput".into();
            plan.selected_translator_family = TranslatorFamily::XboxStyle;
            plan.backend_open_context = BackendOpenContext {
                session_id: SessionId::new(7),
                profile_id: ProfileId::from("xbox360"),
                fidelity_tier: FidelityTier::Compatibility,
                backend_level: BackendLevel::Evdev,
                host_platform: HostPlatform::Linux,
            };
            let ctx = prepared_translation_context(&plan, &registry).expect("context");
            let translator = registry.reverse(TranslatorFamily::XboxStyle).expect("translator");
            let event = BackendReverseEvent {
                session_id: SessionId::new(7),
                profile_id: Some(ProfileId::from("xbox360")),
                timestamp: Timestamp::new(1),
                sequence: 1.into(),
                kind: gr_backend_api::BackendReverseEventKind::HidOutputReport,
                target: None,
                payload: BackendReversePayload::Hid { report_id: Some(1), bytes },
            };
            let mut out = SmallVec::<[_; 4]>::new();
            let _ = translator.translate_reverse(&event, &ctx, &mut out);
            assert_declared_outputs(&out, &declared_semantic_outputs("xbox360"));
        }
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
