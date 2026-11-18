//! Optional typed terminal snapshots. No addresses, paths, or provider persistence policy.
use super::{ControllerSession, HidDriver};
use gr_controller_contract::ControlError;
use gr_hid::Protocol;

pub(crate) trait ResourceProtocol: Protocol {
    type Snapshot: Clone;
    fn resource_dirty(&self) -> bool;
    fn snapshot(&self) -> Self::Snapshot;
}
#[derive(Clone, Copy)]
pub(crate) enum Disposition {
    KeepSnapshot,
    Discard,
}
pub(crate) enum Resolution<S> {
    Unresolved,
    Kept(S),
    Discarded,
}
/// An opt-in package wrapper. R=() is the zero-size stateless control case;
/// existing production families continue storing `ControllerSession` directly.
pub(crate) struct ResourceSession<D: HidDriver, R = (), P = (), W = ()> {
    pub(crate) session: ControllerSession<D, P, W>,
    resolution: R,
}
impl<D: HidDriver, P, W> ResourceSession<D, (), P, W> {
    pub(crate) fn plain(session: ControllerSession<D, P, W>) -> Self {
        Self {
            session,
            resolution: (),
        }
    }
}
impl<D: HidDriver, P, W>
    ResourceSession<D, Resolution<<D::Hid as ResourceProtocol>::Snapshot>, P, W>
where
    D::Hid: ResourceProtocol,
{
    pub(crate) fn new(session: ControllerSession<D, P, W>) -> Result<Self, ControlError> {
        if session.optional_protocol().is_none() {
            return Err(ControlError::UnsupportedControl {
                control: "typed resource protocol unavailable",
            });
        }
        Ok(Self {
            session,
            resolution: Resolution::Unresolved,
        })
    }
    pub(crate) fn resource_dirty(&self) -> bool {
        self.session
            .optional_protocol()
            .expect("resource backend is fixed at construction")
            .resource_dirty()
    }
    /// Closing transport and deciding resource disposition are separate actions.
    /// The first explicit terminal choice wins; no Drop implementation exports.
    pub(crate) fn resolve(
        &mut self,
        choice: Disposition,
    ) -> Result<Option<&<D::Hid as ResourceProtocol>::Snapshot>, ControlError> {
        if !self.session.is_closed() {
            return Err(ControlError::UnsupportedControl {
                control: "resource disposition requires terminal close",
            });
        }
        if matches!(self.resolution, Resolution::Unresolved) {
            self.resolution = match choice {
                Disposition::KeepSnapshot => Resolution::Kept(
                    self.session
                        .optional_protocol()
                        .expect("fixed resource backend")
                        .snapshot(),
                ),
                Disposition::Discard => Resolution::Discarded,
            };
        }
        Ok(match &self.resolution {
            Resolution::Kept(snapshot) => Some(snapshot),
            Resolution::Unresolved | Resolution::Discarded => None,
        })
    }
}
