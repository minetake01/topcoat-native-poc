//! Minimal native runtime for the Topcoat-to-WinUI proof of concept.
//!
//! Parsing belongs to Topcoat itself (`topcoat-view-grammar`). This crate only
//! supplies the state and event vocabulary needed by the WinUI lowering.

pub use topcoat_native_macro::view;
pub use windows_reactor;

use windows_reactor::SetState;

/// A Topcoat-shaped reactive signal backed by a `windows-reactor` state slot.
#[derive(Clone)]
pub struct Signal<T>
where
    T: Clone + PartialEq + 'static,
{
    current: T,
    setter: SetState<T>,
}

impl<T> Signal<T>
where
    T: Clone + PartialEq + 'static,
{
    #[doc(hidden)]
    pub fn new(current: T, setter: SetState<T>) -> Self {
        Self { current, setter }
    }

    /// Returns the value from the current render pass.
    pub fn get(&self) -> T {
        self.current.clone()
    }

    /// Schedules a state update and a WinUI tree reconciliation.
    pub fn set(&self, value: T) {
        self.setter.call(value);
    }
}

/// Event value supplied to Topcoat-style event closures.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Event {
    pub target: EventTarget,
}

impl Event {
    #[doc(hidden)]
    pub fn click() -> Self {
        Self::default()
    }

    #[doc(hidden)]
    pub fn input(value: String) -> Self {
        Self {
            target: EventTarget { value },
        }
    }
}

/// The portable subset of a DOM event target used by this PoC.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventTarget {
    pub value: String,
}

pub mod prelude {
    pub use crate::{Event, Signal, view};
    pub use windows_reactor::{App, Element, RenderCx, Result};
}
