// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! Triggers

use libsigrok_sys::sigrok as sr;

use sr::{sr_trigger, sr_trigger_stage};

use crate::device::Channel;
use crate::sr_try;
use crate::types::SrError;

/// Logic analyzer trigger.
///
/// A **trigger** is defined as a succession of **stages** that define
/// **matches** that must be matched to start or stops the data capture.
///
/// A **stage** is a collection of possible **matches** that, when
/// any of them are fulfilled, advances the trigger to the next stage.
/// The data capture will start only after all stages have been fulfilled.
///
/// A **match** is the actual event, i.e., an electrical signal in a certain
/// state or with a certain value.
#[derive(Debug, Clone)]
pub struct Trigger {
    /// Raw C-FFI pointer to the `sr_trigger` struct.
    p_trigger: *mut sr_trigger,
    /// Raw C-FFI pointer to the trigger stages
    p_stages: Vec<*mut sr_trigger_stage>,
    /// Trigger's name.
    pub name: String,
}

impl Trigger {
    /// Creates a new trigger with a single stage and a single match event.
    pub fn new(name: String, events: Vec<(&Channel, TriggerEvent)>) -> Result<Self, SrError> {
        let cstring_name = std::ffi::CString::new(name.clone()).map_err(|_| SrError::SrNull)?;
        let p_trigger: *mut sr_trigger =
            unsafe { sr::sr_trigger_new(cstring_name.as_ptr().cast_mut()) };

        let mut trigger = Self {
            p_trigger: p_trigger,
            name: name,
            p_stages: Vec::new(),
        };

        trigger.add_match(events)?;

        Ok(trigger)
    }

    /// Returns the raw C-FFI pointer.
    pub fn get_pointer(&self) -> *mut sr_trigger {
        self.p_trigger
    }

    /// Adds a new match for the trigger.
    ///
    /// Adding a match implies two things:
    ///
    /// 1. A new "stage" is added to the trigger, i.e., all the previous
    /// conditions for the trigger must be fulfilled before this new match.
    ///
    /// 2. All the conditions in the match event must be fulfilled (like a
    /// logical AND).
    pub fn add_match(&mut self, events: Vec<(&Channel, TriggerEvent)>) -> Result<(), SrError> {
        let p_stage: *mut sr_trigger_stage = unsafe { sr::sr_trigger_stage_add(self.p_trigger) };

        for (channel, trigger_event) in events {
            let value: f32 = match trigger_event {
                TriggerEvent::Over(f) | TriggerEvent::Under(f) => f,
                _ => 0.0,
            };

            sr_try!(sr::sr_trigger_match_add(
                p_stage,
                channel.get_pointer(),
                trigger_event.into(),
                value,
            ));
        }

        self.p_stages.push(p_stage);
        Ok(())
    }
}

impl Drop for Trigger {
    fn drop(&mut self) {
        //unsafe{sr::sr_trigger_free(self.p_trigger)};
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
