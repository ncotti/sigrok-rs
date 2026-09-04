//! Triggers

use std::ptr::null_mut;

use libsigrok_sys::sigrok::{sr_trigger, sr_trigger_stage};

use libsigrok_sys::sigrok as sr;

use crate::device::Channel;
use crate::sr_try;
use crate::types::SrError;

/// Logic analyzer trigger.
///
/// A **trigger** is defined as a succession of **stages** that define
/// **events** that must be matched to start or stops the data capture.
///
/// A **stage** is a collection of possible events **matches** that, when
/// any of them are fulfilled, advances the trigger to the next stage.
/// The data capture will start only after all stages have been fulfilled.
///
/// A **match** is the actual event, e.g., an electrical signal having a
/// rising edge or a low or high value.
#[derive(Debug, Clone)]
pub struct Trigger {
    /// Raw FFI C pointer to the `sr_trigger` struct.
    p_trigger: *mut sr_trigger,
    /// Trigger's name.
    name: String,
    /// Trigger's stages.
    stages: Vec<TriggerStage>,
}

impl Trigger {
    /// Creates a new trigger with a single stage and a single match event.
    pub fn new(name: String, channel: &Channel, event: TriggerEvent) -> Result<Self, SrError> {
        let p_trigger: *mut sr_trigger =
            unsafe { sr::sr_trigger_new(name.as_ptr().cast_mut().cast()) };

        let stages: Vec<TriggerStage> = vec![TriggerStage::new(p_trigger, channel, event)?];

        Ok(Self {
            p_trigger: p_trigger,
            name: name,
            stages: stages,
        })
    }

    pub fn get_pointer(&self) -> *mut sr_trigger {
        self.p_trigger
    }

    // pub fn add_stage() -> Result<(), SrError> {

    // }

    // pub fn add_match() -> Result<(), SrError> {

    // }
}

impl Drop for Trigger {
    fn drop(&mut self) {
        //unsafe{sr::sr_trigger_free(self.p_trigger)};
    }
}

/// Trigger stage.
///
/// For a trigger to actually be triggered, all the stages must be fulfilled
/// in order.
#[derive(Debug, Clone)]
pub struct TriggerStage {
    /// Raw FFI C pointer.
    p_stage: *mut sr_trigger_stage,

    /// Indicates the order in which the stages need to be
    /// fulfilled for the trigger to activate, starting from 0.
    order: i32,

    /// List of matches associated with this stage. If any of these events
    /// occurs, then the stage is considered fulfilled.
    matches: Vec<TriggerMatch>,
}

impl TriggerStage {
    /// Create a new stage for the given trigger.
    ///
    /// This function will panic if `p_trigger` is null
    pub fn new(
        p_trigger: *mut sr_trigger,
        channel: &Channel,
        event: TriggerEvent,
    ) -> Result<Self, SrError> {
        if p_trigger == null_mut() {
            panic!("TriggerStage::new(), received trigger was NULL.");
        }

        let p_stage: *mut sr_trigger_stage = unsafe { sr::sr_trigger_stage_add(p_trigger) };

        let matches: Vec<TriggerMatch> = vec![TriggerMatch::new(p_stage, channel, event)?];

        Ok(Self {
            p_stage: p_stage,
            order: unsafe { (*p_stage).stage },
            matches: matches,
        })
    }
}

/// Trigger match.
///
/// Holds the channel and the event that will cause the trigger's stage to be
/// completed.
#[derive(Debug, Clone)]
pub struct TriggerMatch {
    /// Channel index where the event will be overseen.
    channel_index: i32,

    /// Event that will cause a "match".
    event: TriggerEvent,
}

impl TriggerMatch {
    /// Creates a new trigger match.
    ///
    /// A *match* is associated to a trigger's stage, and is defined by an
    /// event that must occur on one of the device's channels.
    pub fn new(
        p_stage: *mut sr_trigger_stage,
        channel: &Channel,
        event: TriggerEvent,
    ) -> Result<Self, SrError> {
        let value: f32 = match event {
            TriggerEvent::Over(f) | TriggerEvent::Under(f) => f,
            _ => 0.0,
        };

        sr_try!(sr::sr_trigger_match_add(
            p_stage,
            channel.get_pointer(),
            event.into(),
            value
        ));

        Ok(TriggerMatch {
            channel_index: channel.get_index(),
            event: event,
        })
    }
}

/// Possible events that may activate a trigger.
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum TriggerEvent {
    /// Electrical level zero.
    Zero = 1,
    /// Electrical level one.
    One = 2,
    /// Rising edge.
    Rising = 3,
    /// Falling edge.
    Falling = 4,
    /// Any edge (rising or falling).
    Edge = 5,
    /// Over the given value.
    Over(f32) = 6,
    /// Under the given value.
    Under(f32) = 7,
}

impl Into<i32> for TriggerEvent {
    fn into(self) -> i32 {
        match self {
            TriggerEvent::Zero => 1,
            TriggerEvent::One => 2,
            TriggerEvent::Rising => 3,
            TriggerEvent::Falling => 4,
            TriggerEvent::Edge => 5,
            TriggerEvent::Over(_) => 6,
            TriggerEvent::Under(_) => 7,
        }
    }
}
