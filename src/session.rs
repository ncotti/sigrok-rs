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
    output_module::OutputModule,
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
    device: Device,
    // input: InputModule,
    /// Output module
    output: Arc<Mutex<OutputModule>>,
    /// Handle for the sigrok thread.
    thread_handle: Option<thread::JoinHandle<()>>,
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
            device.get_pointer(),
            p_session,
        )?));

        let session = Self {
            p_context: p_context,
            p_session: p_session,
            device: device,
            output: output,
            thread_handle: None,
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

    pub fn set_trigger(&self, event: TriggerEvent) -> Result<(), SrError> {
        let trigger: Trigger = Trigger::new(
            String::from("name"),
            self.device.get_channel("0").unwrap(),
            event,
        )?;
        sr_try!(sr::sr_session_trigger_set(
            self.p_session,
            trigger.get_pointer()
        ));
        Ok(())
    }

    /// Runs until the trigger or "ms" have passed
    /// Blocking
    pub fn run(&mut self) -> Result<(), SrError> {
        sr_try!(sr::sr_session_datafeed_callback_add(
            self.p_session,
            Some(my_callback),
            self.output.lock().unwrap().get_pointer().cast_mut().cast(),
        ));
        sr_try!(sr::sr_session_start(self.p_session));
        sr_try!(sr::sr_session_run(self.p_session));
        self.stop()?;
        Ok(())
    }

    /// Runs the session for `timeout` time and blocks until the time passes.
    ///
    /// Timing is not precise, and the session may run for more or less time.
    pub fn run_timeout(&mut self, timeout: Duration) -> Result<Vec<u8>, SrError> {
        self.start()?;
        thread::sleep(timeout);
        self.stop()?;
        Ok(self.output.lock().unwrap().data.clone())
    }

    pub fn run_samples(&mut self, samples: u64, timeout: Duration) -> Result<Vec<u8>, SrError> {
        self.output.lock().unwrap().max_samples = samples;
        self.start()?;
        let timer = std::time::Instant::now();

        while timer.elapsed() < timeout {
            let lock = self.output.try_lock();
            let current_samples = if lock.is_ok() {
                lock.unwrap().samples
            } else {
                0
            };
            if current_samples == samples {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        self.stop()?;

        Ok(self.output.lock().unwrap().data.clone())
    }

    /// Starts the session in a new thread.
    pub fn start(&mut self) -> Result<(), SrError> {
        self.output.lock().unwrap().data = Vec::new();
        self.output.lock().unwrap().samples = 0;
        let p_data: *mut c_void =
            (&mut self.output as *mut Arc<Mutex<OutputModule>>) as *mut c_void;
        sr_try!(sr::sr_session_datafeed_callback_add(
            self.p_session,
            Some(my_callback),
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
    pub fn stop(&mut self) -> Result<(), SrError> {
        if unsafe { sr::sr_session_is_running(self.p_session) } == 1 {
            sr_try!(sr::sr_session_stop(self.p_session));
        }
        if self.thread_handle.is_some() {
            let handle = self.thread_handle.take().unwrap();
            handle.join().expect("Thread should be joinable.");
        }

        sr_try!(sr::sr_session_datafeed_callback_remove_all(self.p_session));
        Ok(())
    }

    pub fn set_output(&mut self, id: &str, filename: impl AsRef<Path>) -> Result<(), SrError> {
        let filename: &Path = filename.as_ref();
        *(self.output.lock().unwrap()) =
            OutputModule::new(id, filename, self.device.get_pointer(), self.p_session)?;
        Ok(())
    }
}

extern "C" fn stopped_callback(cb_data: *mut std::ffi::c_void) {
    println!("Session stopped");
}

extern "C" fn my_callback(
    p_device: *const sr_dev_inst,
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
        PacketType::Header => {
            // TODO, see what to do here
            //println!("{}", HeaderPacket::from(packet.payload));
        }
        PacketType::End => {}
        PacketType::Meta => {}
        PacketType::Trigger => {}
        PacketType::Logic => {
            // The user requested for the loop to run until "max_samples"
            // have been received
            if (output.samples < output.max_samples) || (output.max_samples == 0) {
                let mut logic_packet: LogicPacket = LogicPacket::from(packet.payload);

                if output.max_samples > 0 {
                    let packet_data_len =
                        if (output.samples + logic_packet.data.len() as u64) > output.max_samples {
                            output.max_samples - output.samples
                        } else {
                            logic_packet.data.len() as u64
                        };
                    logic_packet.data.resize(packet_data_len as usize, 0);
                }

                dbg!(logic_packet.data.len());
                dbg!(output.max_samples);
                output.samples += logic_packet.data.len() as u64;
                output.data.extend(logic_packet.data);

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
                            std::slice::from_raw_parts(
                                gstring.str_ as *const u8,
                                gstring.len as usize,
                            )
                        };
                        file.write(bytes).unwrap();
                    }
                    //unsafe { glib::ffi::g_string_free(p_gstring as *mut GString, 1) };
                }

                // TODO, fix the reason why putting a sleep breaks everything in the callback
                //thread::sleep(Duration::from_millis(1));
            }
        }
        PacketType::FrameBegin => {}
        PacketType::FrameEnd => {}
        PacketType::Analog => {}
    };
}
