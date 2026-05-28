//! Linux transport provider foundation for `virtualgamepad`.
//!
//! Phase 10 prep: stub types only. The crate declares
//! `LinuxTransportUsbBackendFactory`,
//! `LinuxTransportBluetoothBackendFactory`, and
//! `LinuxTransportBackendSession` shapes so workspace inventory,
//! planner, and `support-report` code can refer to them ahead of the
//! Phase 10 implementation PR that lands the enumeration and protocol
//! state machines, USB and Bluetooth packet models, and the
//! transport-tier translators.

#![allow(clippy::module_name_repetitions)]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use gr_backend_api::{
    BackendDiagnostics, BackendError, BackendFactory, BackendFrame, BackendInventoryEntry,
    BackendOpenContext, BackendRealizationRequest, BackendReverseEventSink, BackendSession,
    BackendState, BackendSupportReport, EventReadiness, SupportLevel,
};
use gr_core::{BackendFamily, BackendId, BackendLevel, ProfileId, SessionId};
use gr_runtime_model::HostPlatform;

const PHASE_10_PENDING_NOTE: &str = "phase-10 implementation pending: enumeration / protocol state machines, USB and \
     Bluetooth packet models, and transport-tier translators land in the Phase 10 \
     implementation PR";

const USB_BACKEND_ID: &str = "linux-transport-usb";
const BLUETOOTH_BACKEND_ID: &str = "linux-transport-bluetooth";

pub struct LinuxTransportUsbBackendFactory {
    backend_id: BackendId,
    notes: Vec<String>,
}

impl Default for LinuxTransportUsbBackendFactory {
    fn default() -> Self {
        Self {
            backend_id: BackendId::from(USB_BACKEND_ID),
            notes: vec![PHASE_10_PENDING_NOTE.to_string()],
        }
    }
}

impl LinuxTransportUsbBackendFactory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl BackendFactory for LinuxTransportUsbBackendFactory {
    fn backend_id(&self) -> BackendId {
        self.backend_id.clone()
    }

    fn family(&self) -> BackendFamily {
        BackendFamily::LinuxTransportUsb
    }

    fn inventory_entry(&self) -> BackendInventoryEntry {
        BackendInventoryEntry {
            backend_id: self.backend_id(),
            family: self.family(),
            level: BackendLevel::Transport,
            host_platform: HostPlatform::Linux,
            supported_fidelity_tiers: Vec::new(),
            notes: self.notes.clone(),
        }
    }

    fn can_realize(&self, _request: &BackendRealizationRequest) -> BackendSupportReport {
        BackendSupportReport {
            forward_support: SupportLevel::None,
            reverse_support: SupportLevel::None,
            supported_output_functions: Vec::new(),
            unsupported_output_functions: Vec::new(),
            notes: self.notes.clone(),
        }
    }

    fn open_session(
        &self,
        _context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_10_PENDING_NOTE.to_string(),
        })
    }
}

pub struct LinuxTransportBluetoothBackendFactory {
    backend_id: BackendId,
    notes: Vec<String>,
}

impl Default for LinuxTransportBluetoothBackendFactory {
    fn default() -> Self {
        Self {
            backend_id: BackendId::from(BLUETOOTH_BACKEND_ID),
            notes: vec![PHASE_10_PENDING_NOTE.to_string()],
        }
    }
}

impl LinuxTransportBluetoothBackendFactory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl BackendFactory for LinuxTransportBluetoothBackendFactory {
    fn backend_id(&self) -> BackendId {
        self.backend_id.clone()
    }

    fn family(&self) -> BackendFamily {
        BackendFamily::LinuxTransportBluetooth
    }

    fn inventory_entry(&self) -> BackendInventoryEntry {
        BackendInventoryEntry {
            backend_id: self.backend_id(),
            family: self.family(),
            level: BackendLevel::Transport,
            host_platform: HostPlatform::Linux,
            supported_fidelity_tiers: Vec::new(),
            notes: self.notes.clone(),
        }
    }

    fn can_realize(&self, _request: &BackendRealizationRequest) -> BackendSupportReport {
        BackendSupportReport {
            forward_support: SupportLevel::None,
            reverse_support: SupportLevel::None,
            supported_output_functions: Vec::new(),
            unsupported_output_functions: Vec::new(),
            notes: self.notes.clone(),
        }
    }

    fn open_session(
        &self,
        _context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_10_PENDING_NOTE.to_string(),
        })
    }
}

pub struct LinuxTransportBackendSession {
    session_id: SessionId,
    backend_id: BackendId,
    family: BackendFamily,
    profile_id: ProfileId,
    state: BackendState,
}

impl LinuxTransportBackendSession {
    #[must_use]
    pub fn new_usb(session_id: SessionId, profile_id: ProfileId) -> Self {
        Self {
            session_id,
            backend_id: BackendId::from(USB_BACKEND_ID),
            family: BackendFamily::LinuxTransportUsb,
            profile_id,
            state: BackendState::NotOpen,
        }
    }

    #[must_use]
    pub fn new_bluetooth(session_id: SessionId, profile_id: ProfileId) -> Self {
        Self {
            session_id,
            backend_id: BackendId::from(BLUETOOTH_BACKEND_ID),
            family: BackendFamily::LinuxTransportBluetooth,
            profile_id,
            state: BackendState::NotOpen,
        }
    }

    #[must_use]
    pub fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }
}

impl BackendSession for LinuxTransportBackendSession {
    fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn open(&mut self) -> Result<(), BackendError> {
        self.state = BackendState::Failed;
        Err(BackendError::OpenFailed {
            reason: PHASE_10_PENDING_NOTE.to_string(),
        })
    }

    fn send(&mut self, _frame: BackendFrame) -> Result<(), BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_10_PENDING_NOTE.to_string(),
        })
    }

    fn drain_reverse_events(
        &mut self,
        _out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError> {
        Err(BackendError::Unsupported {
            reason: PHASE_10_PENDING_NOTE.to_string(),
        })
    }

    fn readiness(&self) -> EventReadiness {
        EventReadiness::NoReverseEvents
    }

    fn diagnostics(&self) -> BackendDiagnostics {
        BackendDiagnostics {
            backend_id: self.backend_id.clone(),
            family: self.family,
            state: self.state,
            frames_sent: 0,
            reverse_events_drained: 0,
            write_failures: 0,
            last_error: None,
            vendor_counters: BTreeMap::new(),
        }
    }

    fn close(&mut self) -> Result<(), BackendError> {
        self.state = BackendState::Closed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_factory_inventory_reports_transport_level() {
        let factory = LinuxTransportUsbBackendFactory::new();
        let entry = factory.inventory_entry();
        assert_eq!(entry.backend_id, BackendId::from(USB_BACKEND_ID));
        assert_eq!(entry.family, BackendFamily::LinuxTransportUsb);
        assert_eq!(entry.level, BackendLevel::Transport);
        assert_eq!(entry.host_platform, HostPlatform::Linux);
        assert!(entry.supported_fidelity_tiers.is_empty());
        assert!(
            entry
                .notes
                .iter()
                .any(|note| note.contains("phase-10 implementation pending"))
        );
    }

    #[test]
    fn bluetooth_factory_inventory_reports_transport_level() {
        let factory = LinuxTransportBluetoothBackendFactory::new();
        let entry = factory.inventory_entry();
        assert_eq!(entry.backend_id, BackendId::from(BLUETOOTH_BACKEND_ID));
        assert_eq!(entry.family, BackendFamily::LinuxTransportBluetooth);
        assert_eq!(entry.level, BackendLevel::Transport);
    }

    #[test]
    fn factories_refuse_realization_until_phase_10_lands() {
        let usb = LinuxTransportUsbBackendFactory::new();
        let bluetooth = LinuxTransportBluetoothBackendFactory::new();
        for support in [
            usb.can_realize(&dummy_request()),
            bluetooth.can_realize(&dummy_request()),
        ] {
            assert_eq!(support.forward_support, SupportLevel::None);
            assert_eq!(support.reverse_support, SupportLevel::None);
        }
    }

    fn dummy_request() -> BackendRealizationRequest {
        BackendRealizationRequest {
            profile_id: ProfileId::from("dualsense"),
            requested_goal: gr_runtime_model::EmulationGoal::HardwareFaithful,
            requested_fidelity_tier: gr_core::FidelityTier::HardwareFaithful,
            host_platform: HostPlatform::Linux,
            required_output_functions: Vec::new(),
        }
    }
}
