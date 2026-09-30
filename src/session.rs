// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026  Nicolas Gabriel Cotti

//! # Data capture session
//!
//! A session encompass all the required components for a logic analyzer to do
//! a data capture, that is:
//!
//! 1. Scans for connected devices.
//! 2. Connects to the plugged **device** using its respective **driver**.
//! 3. Sets all parameters for the session and for the device. E.g.: sample rate,
//! how many samples to capture, output format, etc.
//! 4. Runs the data capture.
//! 5. Returns the captured samples.
//!
//! The normal workflow starts by creating a session attached to a device, and
//! then running the data capture.
//!
//! ```rust
//! use sigrok_rs::Session;
//!
//! let mut session: Session = Session::try_from("demo").unwrap();
//! session.run().unwrap();
//! ```
//! ## Running methods
//!
//! There are three ways to perform a data capture:
//!
//! 1. Run until a timeout is reached.
//!
//! ```rust
//! use sigrok_rs::Session;
//! use std::time::Duration;
//!
//! let mut session: Session = Session::try_from("demo").unwrap();
//! session.set_timeout(Duration::from_millis(10)).unwrap();
//! session.set_samplerate(100000).unwrap();
//! let data: Vec<u8> = session.run().unwrap();
//!
//! assert!(data.len() == 100000 * 10 / 1000);
//! ```
//!
//! 2. Run until a certain amount of samples have been captured:
//!
//! ```rust
//! use sigrok_rs::Session;
//!
//! let mut session: Session = Session::try_from("demo").unwrap();
//! let data: Vec<u8> = session.run_samples(8).unwrap();
//!
//! assert!(data.len() == 8);
//! ```
//!
//! 3. Run when a trigger condition is met. In this case, the session will
//! return the number of samples requested after the trigger condition is met,
//! including the trigger condition itself; or it will fail if the trigger is
//! not fulfilled before the timeout.
//!
//! ```rust
//! use sigrok_rs::{Session, TriggerEvent};
//!
//! let mut session: Session = Session::try_from("demo").unwrap();
//! let events = vec![("D0", TriggerEvent::Rising)];
//! session.set_trigger(events).unwrap();
//! let data: Vec<u8> = session.run_samples(10).unwrap();
//!
//! assert!(data.len() == 10);
//! ```
//!
//! ## Session configuration
//!
//! The following things can be configured for the session:
//!
//! * **Output file format**: Besides returning the data capture as a `Vec<u8>`
//! vector, the session can be stored in a file.
//!
//! * **Timeout**: The session will run for at most the provided *timeout*.
//!
//! * **Log level**: Messages printed by the C libsigrok library.
//!
//! An example changing all the aforementioned configurations is shown below:
//!
//! ```rust
//! use sigrok_rs::{Session, LogLevel};
//! use std::time::Duration;
//!
//! let mut session: Session = Session::try_from("demo").unwrap();
//! session.set_output("ascii", "output_file.txt").unwrap();
//! session.set_timeout(Duration::from_millis(10)).unwrap();
//! session.set_log_level(LogLevel::LogWarn).unwrap();
//! ```
//!
//! ## Using triggers
//!
//! Data acquisition can be started after a certain *trigger* is met. Triggers
//! are specified as a vector of `("channel_id", TriggerEvent)`. All conditions
//! must be met simultaneously in the same sample for the trigger to be
//! activated, and the data acquisition includes the sample that caused the
//! trigger.
//!
//! It is also possible to define several *trigger stages*. In this case, data
//! capture will start after each of the stages' conditions are fulfilled in
//! the order they were added to the session, in consecutive samples.
//!
//! ```rust
//! use sigrok_rs::{Session, TriggerEvent};
//!
//! let mut session = Session::try_from("demo").unwrap();
//! let events = vec![
//!     ("D0", TriggerEvent::One),
//!     ("D1", TriggerEvent::One),
//! ];
//! session.set_trigger(events.clone()).unwrap();
//! session.add_trigger_stage(events.clone()).unwrap();
//! session.add_trigger_stage(events.clone()).unwrap();
//!
//! let data = session.run_samples(10).unwrap();
//! assert!(data[0] & 0b11 == 0b11);
//! ```

use std::{
    ffi::c_void,
    fs::OpenOptions,
    ptr::{null, null_mut},
};

use std::sync::{Arc, Mutex};

use std::io::Write;

use std::path::Path;

use libsigrok_sys::sigrok as sr;

use crate::{
    Device, LogLevel, SrError,
    device::Channel,
    output_module::{OutputModule, OutputModuleInfo},
    packets::LogicPacket,
    sr_try,
    trigger::{Trigger, TriggerEvent},
    types::PacketType,
};
use sr::{_GString, sr_context, sr_datafeed_packet, sr_dev_inst, sr_session};

use std::thread;
use std::time::Duration;

/// A Sigrok session
pub struct Session {
    /// Sigrok library context. This is the value returned when calling
    /// `sr_init()` and freed by `sr_exit()`. It is mandatory for the
    /// library to work.
    p_context: *mut sr_context,
    /// Raw C-FFI to the session
    p_session: *mut sr_session,
    /// Device associated with the session. There can only be one device
    /// per session.
    pub device: Device,
    // input: InputModule,
    /// Output module
    output: Arc<Mutex<OutputModule>>,
    /// Handle for the sigrok thread.
    thread_handle: Option<thread::JoinHandle<()>>,
    /// Output module ID.
    output_id: String,
    /// Output module filename.
    output_filename: String,
    /// Trigger event for the session.
    trigger: Option<Trigger>,
    /// Maximum amount of time a session can run, before being abruptly
    /// interrupted.
    timeout: Duration,
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { sr::sr_session_stop(self.p_session) };
        unsafe { sr::sr_dev_close(self.device.get_pointer()) };
        unsafe { sr::sr_session_dev_remove_all(self.p_session) };
        unsafe { sr::sr_session_destroy(self.p_session) };
        unsafe { sr::sr_exit(self.p_context) };
    }
}

impl TryFrom<&str> for Session {
    type Error = SrError;

    /// Builds a sessions from a device ID, which may be its name, serial
    /// number, driver name, connection ID or model.
    ///
    /// The device will be opened and attached to the session.
    fn try_from(device_id: &str) -> Result<Self, SrError> {
        let mut session = Self::new()?;
        let devices = Device::scan(session.p_context)?;

        let device = devices.into_iter().find(|dev| dev == &device_id);

        if device.is_none() {
            return Err(SrError::SrDeviceNotFound);
        }

        session.device = device.unwrap();
        session.device.open()?;

        sr_try!(sr::sr_session_dev_add(
            session.p_session,
            session.device.get_pointer()
        ));

        Ok(session)
    }
}

impl TryFrom<&String> for Session {
    type Error = SrError;

    fn try_from(device_id: &String) -> Result<Self, SrError> {
        Self::try_from(device_id.as_str())
    }
}

impl TryFrom<String> for Session {
    type Error = SrError;

    fn try_from(device_id: String) -> Result<Self, SrError> {
        Self::try_from(&device_id)
    }
}

impl TryFrom<Device> for Session {
    type Error = SrError;

    fn try_from(device: Device) -> Result<Self, SrError> {
        // Although it could be tempting to use the given Device directly,
        // the scanned device must come from the Session context, which
        // is yet to be created.
        //
        // The default Device::scan() uses a one-time context that is
        // then erased. Therefore, the given device is a dangling reference.
        Self::try_from(device.get_driver_name())
    }
}

impl Session {
    /// Creates a new "empty" session.
    ///
    /// The returned Session object will be plugged to the "demo" device, and
    /// with a "null" output module.
    fn new() -> Result<Self, SrError> {
        sr_try!(sr::sr_log_callback_set_default());

        let mut p_context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut p_context));

        let mut p_session: *mut sr_session = null_mut();
        sr_try!(sr::sr_session_new(p_context, &mut p_session));

        let device = Device::try_from(("demo", p_context))?;

        let output = Arc::new(Mutex::new(OutputModule::new(
            "null",
            "",
            u64::MAX,
            device.get_pointer(),
        )?));

        let session = Self {
            p_context: p_context,
            p_session: p_session,
            device: device,
            output: output,
            thread_handle: None,
            output_id: String::from("null"),
            output_filename: String::new(),
            trigger: None,
            timeout: Duration::from_secs(1),
        };

        Ok(session)
    }

    /// Tries to connect to a single device connected to the host PC and
    /// returns the `Session` object.
    ///
    /// This function will fail if no device is connected, or if there are
    /// more than one hardware device is discovered (not counting the "demo"
    /// device).
    pub fn autoconnect() -> Result<Self, SrError> {
        let devices = Session::scan()?;

        if devices.len() == 1 {
            Err(SrError::SrDeviceNotFound)
        } else if devices.len() > 2 {
            Err(SrError::SrMultipleDevices)
        } else {
            let hardware_device = devices
                .into_iter()
                .find(|dev| dev.get_driver_name() != "demo")
                .expect("There is a second device, which is not the demo one");
            Ok(Session::try_from(hardware_device)?)
        }
    }

    /// Sets the log level for the libsigrok functions.
    pub fn set_log_level(&self, level: LogLevel) -> Result<(), SrError> {
        sr_try!(sr::sr_log_loglevel_set(level as i32));
        Ok(())
    }

    /// Returns a list with all discovered devices currently plugged to the PC.
    ///
    /// The device "demo" is always discovered, so the returned vector will
    /// never be empty.
    pub fn scan() -> Result<Vec<Device>, SrError> {
        let mut p_context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut p_context));

        let devices: Vec<Device> = Device::scan(p_context)?;

        unsafe { sr::sr_exit(p_context) };
        Ok(devices)
    }

    /// Returns a list of all possible output modules.
    ///
    /// Output modules are configured by calling `self.set_output()`.
    pub fn scan_output() -> Result<Vec<OutputModuleInfo>, SrError> {
        let mut p_context: *mut sr_context = null_mut();
        sr_try!(sr::sr_init(&mut p_context));

        let output_modules = OutputModuleInfo::scan();

        unsafe { sr::sr_exit(p_context) };
        Ok(output_modules)
    }

    /// Creates a new trigger for the session.
    ///
    /// The trigger is defined by a set of "channel / event" couples.
    /// All elements in the vector must be fulfilled for the trigger to
    /// activate, like a logical "AND".
    ///
    /// This function will fail if any of the channels' names are incorrect,
    /// or the `events` vector is empty.
    pub fn set_trigger(&mut self, events: Vec<(&str, TriggerEvent)>) -> Result<(), SrError> {
        if events.is_empty() {
            return Err(SrError::SrErrArg);
        }

        let mut channel_events: Vec<(&Channel, TriggerEvent)> = Vec::new();

        for (channel_name, trigger_event) in events {
            let channel = self.device.get_channel(channel_name)?;
            channel_events.push((channel, trigger_event));
        }

        let trigger: Trigger = Trigger::new(channel_events)?;

        self.trigger = Some(trigger);
        Ok(())
    }

    /// Adds a stage to an already existing trigger.
    ///
    /// This is a new condition, which must be fulfilled after all other
    /// previous conditions have been met.
    ///
    /// Stages must occur in consecutive samples for the trigger to activate.
    pub fn add_trigger_stage(&mut self, events: Vec<(&str, TriggerEvent)>) -> Result<(), SrError> {
        if self.trigger.is_none() {
            return Err(SrError::SrErrBug);
        }

        if events.is_empty() {
            return Err(SrError::SrErrArg);
        }

        let mut channel_events: Vec<(&Channel, TriggerEvent)> = Vec::new();

        for (channel_name, trigger_event) in events {
            let channel = self.device.get_channel(channel_name)?;
            channel_events.push((channel, trigger_event));
        }

        self.trigger.as_mut().unwrap().add_match(channel_events)?;
        Ok(())
    }

    /// Runs the session for `timeout` time. Blocks the thread execution.
    pub fn run(&mut self) -> Result<Vec<u8>, SrError> {
        // Although some devices have the "limit_time" option for ending the
        // data acquisition, this is not available for all devices.
        // Therefore, we will use the "limit_samples" and the "samplerate" as
        // a surefire replacement.
        let samples = self.timeout_to_samples(self.timeout)?;
        self.start(samples, samples)?;
        self.read()
    }

    /// Runs the session until a given amount of samples are retrieved.
    /// Blocks the thread execution.
    ///
    /// If a trigger condition was set before `timeout`, this function will
    /// return `samples_after_trigger` samples.
    ///
    /// The output file will hold all samples and a marker showing where the
    /// trigger condition happened.
    pub fn run_samples(&mut self, samples: u64) -> Result<Vec<u8>, SrError> {
        let samples_until_timeout = if self.trigger.is_some() {
            self.timeout_to_samples(self.timeout)?
        } else {
            samples
        };

        self.start(samples, samples_until_timeout)?;
        self.read()
    }

    /// Runs the session for `timeout` time. Spawns a new thread running in
    /// the background, so it does not block the calling thread.
    ///
    /// Data can be read later by using `self.read()?`.
    pub fn run_daemon(&mut self) -> Result<(), SrError> {
        let samples = self.timeout_to_samples(self.timeout)?;
        self.start(samples, samples)
    }

    /// Runs the session until a given amount of samples are retrieved. Spawns
    /// a new thread running in the background, so it does not block the
    /// calling thread.
    ///
    /// If a trigger condition was set before `timeout`, this function will
    /// return `samples_after_trigger` samples.
    ///
    /// The output file will hold all samples and a marker showing where the
    /// trigger condition happened.
    pub fn run_samples_daemon(&mut self, samples: u64) -> Result<(), SrError> {
        let samples_until_timeout = if self.trigger.is_some() {
            self.timeout_to_samples(self.timeout)?
        } else {
            samples
        };

        self.start(samples, samples_until_timeout)
    }

    /// Reads output data from a session that has run.
    ///
    /// If there is a session running, it will wait until that session ends.
    ///
    /// If no session was started, it will fail. If a session ran but couldn't
    /// fetch any data, usually because a trigger condition was not met, it
    /// will fail.
    pub fn read(&mut self) -> Result<Vec<u8>, SrError> {
        self.stop(false)?;

        let mut output = self.output.lock().unwrap();

        if output.data.len() == 0 {
            if self.trigger.is_some() {
                return Err(SrError::SrErrTimeout);
            } else {
                return Err(SrError::SrNoData);
            }
        }

        let data = output.data.clone();
        output.data.clear();

        Ok(data)
    }

    /// Sets the default
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), SrError> {
        self.timeout = timeout;
        Ok(())
    }

    /// Using the device's samplerate, converts a time duration into the
    /// equivalent number of samples that should be taken to have that time
    /// pass.
    pub fn timeout_to_samples(&self, timeout: Duration) -> Result<u64, SrError> {
        let samplerate: u64 = self.get_samplerate().unwrap();
        let timeout_samples = samplerate * (timeout.as_millis() as u64) / 1000;
        Ok(timeout_samples)
    }

    /// Forcefully aborts execution for a running session, returning as much
    /// data as could be retrieved.
    pub fn abort(&mut self) -> Result<Vec<u8>, SrError> {
        self.stop(true)?;
        self.read()
    }

    /// Starts the session in a new thread.
    fn start(&mut self, samples_after_trigger: u64, max_samples: u64) -> Result<(), SrError> {
        if self.is_running() {
            return Err(SrError::SrSessionAlreadyRunning);
        }

        *(self.output.lock().unwrap()) = OutputModule::new(
            &self.output_id,
            &self.output_filename,
            samples_after_trigger,
            self.device.get_pointer(),
        )?;

        self.device
            .set_option("limit_samples", &max_samples.to_string())?;

        if self.trigger.is_some() {
            sr_try!(sr::sr_session_trigger_set(
                self.p_session,
                self.trigger.as_ref().unwrap().get_pointer()
            ));
            self.output.lock().unwrap().triggered = false;
        } else {
            // If there is no trigger, assume that it has already been
            // triggered, so data acquisition can start
            self.output.lock().unwrap().triggered = true;
        }

        let p_data: *mut c_void =
            (&mut self.output as *mut Arc<Mutex<OutputModule>>) as *mut c_void;
        sr_try!(sr::sr_session_datafeed_callback_add(
            self.p_session,
            Some(datafeed_callback),
            p_data,
        ));
        sr_try!(sr::sr_session_stopped_callback_set(
            self.p_session,
            Some(stopped_callback),
            null_mut()
        ));

        sr_try!(sr::sr_session_start(self.p_session));
        let th_p_session: usize = self.p_session as usize;
        self.thread_handle = Some(thread::spawn(move || {
            unsafe { sr::sr_session_run(th_p_session as *mut sr_session) };
        }));
        Ok(())
    }

    /// Stops a running session.
    ///
    /// Sessions should be started with the `start()` method. This function
    /// will not return an error if the session was not running.
    fn stop(&mut self, force: bool) -> Result<(), SrError> {
        if self.is_running() && force {
            sr_try!(sr::sr_session_stop(self.p_session));
        }
        if self.thread_handle.is_some() {
            let handle = self.thread_handle.take().unwrap();
            handle.join().expect("Thread should be joinable.");
        }

        sr_try!(sr::sr_session_datafeed_callback_remove_all(self.p_session));
        Ok(())
    }

    /// Sets the output module parameters.
    ///
    /// The output module must be created right before starting the session,
    /// because it does not get updated if a device parameter is changed
    /// after being created. Therefore, this function will not check
    /// whether the provided ID and filename are valid ones.
    pub fn set_output(&mut self, id: &str, filename: impl AsRef<Path>) -> Result<(), SrError> {
        let filename: &Path = filename.as_ref();
        self.output_filename = filename.to_string_lossy().to_string();
        self.output_id = id.to_string();
        Ok(())
    }

    /// Sets the device's sample rate.
    pub fn set_samplerate(&mut self, samplerate: u64) -> Result<(), SrError> {
        self.device
            .set_option("samplerate", &samplerate.to_string())
    }

    /// Returns the device's sample rate
    pub fn get_samplerate(&self) -> Result<u64, SrError> {
        let result = self.device.get_option("samplerate");

        if result.is_ok() {
            Ok(result.unwrap().parse().unwrap())
        } else {
            Err(result.unwrap_err())
        }
    }

    /// Returns whether there is an active session or not.
    pub fn is_running(&self) -> bool {
        (unsafe { sr::sr_session_is_running(self.p_session) } == 1)
    }
}

/// This function will get called whenever a session is stopped.
///
/// As of right now, it is left as a placeholder.
extern "C" fn stopped_callback(_cb_data: *mut std::ffi::c_void) {}

/// This function is called whenever a new packet is received while a session
/// is running. This function gets executed in a different thread than the
/// main one.
extern "C" fn datafeed_callback(
    _p_device: *const sr_dev_inst,
    p_packet: *const sr_datafeed_packet,
    p_data: *mut std::ffi::c_void,
) {
    // Checking that p_packet is not null before deref
    if p_packet == null() {
        return;
    }
    let packet = unsafe { *p_packet };
    let packet_type = PacketType::try_from(packet.type_).unwrap();

    // Checking that p_data is not null before deref
    if p_data == null_mut() {
        return;
    }

    let output: &Arc<Mutex<OutputModule>> =
        unsafe { &*(p_data as *const Arc<Mutex<OutputModule>>) };
    let mut output = output.lock().unwrap();

    match packet_type {
        PacketType::Header => {}
        PacketType::End => {}
        PacketType::Meta => {}
        PacketType::Trigger => {
            output.triggered = true;
        }
        PacketType::Logic => {
            if output.triggered && (output.data.len() < (output.max_samples as usize)) {
                let mut logic_packet: LogicPacket = LogicPacket::from(packet.payload);

                if logic_packet.data.len() + output.data.len() > output.max_samples as usize {
                    logic_packet
                        .data
                        .resize(output.max_samples as usize - output.data.len(), 0);
                }
                output.data.extend(logic_packet.data);
            }
        }
        PacketType::FrameBegin => {}
        PacketType::FrameEnd => {}
        PacketType::Analog => {}
    };

    let mut p_gstring: *mut _GString = null_mut();
    assert!(
        SrError::SrOk as i32
            == unsafe {
                sr::sr_output_send(
                    output.get_pointer(),
                    p_packet,
                    std::ptr::addr_of_mut!(p_gstring),
                )
            }
    );

    if p_gstring != null_mut() {
        let gstring = unsafe { *p_gstring };
        if gstring.len > 0 {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(output.get_filename())
                .unwrap();

            let bytes = unsafe {
                std::slice::from_raw_parts(gstring.str_ as *const u8, gstring.len as usize)
            };
            file.write(bytes).unwrap();
        }
        unsafe { glib::ffi::g_string_free(p_gstring as *mut glib::ffi::GString, 1) };
    }
}
