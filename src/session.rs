//! Session

use std::{
    ffi::c_void,
    fs::OpenOptions,
    ptr::{null, null_mut},
};

use std::sync::{Arc, Mutex};

use std::io::Write;

use std::path::Path;

use glib::LogLevel;
use libsigrok_sys::sigrok::{self as sr, _GString, sr_datafeed_packet, sr_dev_inst};

use crate::{
    Device, SrError,
    device::Channel,
    output_module::{OutputModule, OutputModuleInfo},
    packets::LogicPacket,
    sr_try,
    trigger::{Trigger, TriggerEvent},
    types::PacketType,
};
use sr::{sr_context, sr_session};

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
            p_session,
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

    /// Sets the log level for the libsigrok functions.
    pub fn set_log_level(&self, level: LogLevel) -> Result<(), SrError> {
        sr_try!(sr::sr_log_loglevel_set(level as i32));
        Ok(())
    }

    /// Returns a list with the driver names of all discovered devices
    /// currently plugged to the PC.
    ///
    /// The driver name can be used to start a new session.
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
    pub fn set_trigger(
        &mut self,
        name: &str,
        events: Vec<(&str, TriggerEvent)>,
    ) -> Result<(), SrError> {
        if events.is_empty() {
            return Err(SrError::SrErrArg);
        }

        let mut channel_events: Vec<(&Channel, TriggerEvent)> = Vec::new();

        for (channel_name, trigger_event) in events {
            let channel = self.device.get_channel(channel_name)?;
            channel_events.push((channel, trigger_event));
        }

        let trigger: Trigger = Trigger::new(String::from(name), channel_events)?;

        self.trigger = Some(trigger);
        Ok(())
    }

    /// Adds a stage to an already existing trigger.
    ///
    /// This is a new condition, which must be fulfilled after all other
    /// previous conditions have been met.
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

    /// Runs the session for `timeout` time.
    pub fn run(&mut self) -> Result<Vec<u8>, SrError> {
        // Although some devices have the "limit_time" option for ending the
        // data acquisition, this is not available for all devices.
        // Therefore, we will use the "limit_samples" and the "samplerate" as
        // a surefire replacement.
        let samples = self.timeout_to_samples(self.timeout)?;
        self.start(samples, samples)?;
        self.stop(false)?;

        Ok(self.output.lock().unwrap().data.clone())
    }

    /// Runs the session until a given amount of samples are retrieved.
    ///
    /// If a trigger condition was set before `timeout`, this function will
    /// return `samples_after_trigger` samples.
    ///
    /// The output file will holds all samples and a marker showing where the
    /// trigger condition happened.
    pub fn run_samples(&mut self, samples: u64) -> Result<Vec<u8>, SrError> {
        let samples_until_timeout = if self.trigger.is_some() {
            self.timeout_to_samples(self.timeout)?
        } else {
            samples
        };

        self.start(samples, samples_until_timeout)?;
        self.stop(false)?;

        let data = self.output.lock().unwrap().data.clone();

        // Timeout reached, the trigger was never met
        if data.len() == 0 && self.trigger.is_some() {
            return Err(SrError::SrErrTimeout);
        }

        Ok(data)
    }

    /// Sets the default
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Using the device's samplerate, converts a time duration into the
    /// equivalent number of samples that should be taken to have that time
    /// pass.
    pub fn timeout_to_samples(&self, timeout: Duration) -> Result<u64, SrError> {
        let samplerate: u64 = self.device.get_option("samplerate")?.parse().unwrap();
        let timeout_samples = samplerate * (timeout.as_millis() as u64) / 1000;
        Ok(timeout_samples)
    }

    /// Starts the session in a new thread.
    fn start(&mut self, samples_after_trigger: u64, max_samples: u64) -> Result<(), SrError> {
        *(self.output.lock().unwrap()) = OutputModule::new(
            &self.output_id,
            &self.output_filename,
            samples_after_trigger,
            self.device.get_pointer(),
            self.p_session,
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
        if unsafe { sr::sr_session_is_running(self.p_session) } == 1 && force {
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
