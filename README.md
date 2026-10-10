# Sigrok-rs

Programmatically manage any logic analyzer. Capture samples from your
device to files or Rust variables, automate measuring and testing
of electrical signals from real hardware and decode bus protocols like SPI, I2C and JTAG.

Provides a Rust-friendly implementation for [libsigrok](https://sigrok.org/wiki/Libsigrok), using the
C-FFI [libsigrok-sys](https://crates.io/crates/libsigrok-sys).

## Example data acquisition

The following example connects to the "demo" device, and captures ten
samples; storing them in the file "data_capture.txt" and in a `Vec<u8>`.

```rust
use sigrok_rs::Session;

let mut session: Session = Session::try_from("demo").unwrap();
session.set_output("bits", "data_capture.txt").unwrap();
let capture_data: Vec<u8> = session.run_samples(10).unwrap();

assert!(capture_data.len() == 10);
```

If a new device is plugged to the PC and its information is unknown, you
can *scan* for connected devices and print their information:

```rust
use sigrok_rs::{Session, Device};

let mut devices: Vec<Device> = Session::scan().unwrap();
for device in &devices {
    println!("{}", device);
}
```
