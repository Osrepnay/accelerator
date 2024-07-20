use std::{ffi::OsString, fs::File};

use evdev_rs::{
    enums::{EventCode, EV_REL, EV_SYN},
    Device, GrabMode, InputEvent, ReadFlag, ReadStatus, UInputDevice,
};
use pico_args::Arguments;

fn factor(sens_multiplier: f64, accel: f64, cap: f64, offset: f64, speed: f64) -> f64 {
    if speed < offset {
        sens_multiplier
    } else {
        sens_multiplier * f64::min(accel.mul_add(speed - offset, 1.0), cap)
    }
}

struct Args {
    sens_mult: f64,
    accel: f64,
    cap: f64,
    offset: f64,
    filename: OsString,
}

fn parse_args(arguments: &mut Arguments) -> Result<Args, pico_args::Error> {
    Ok(Args {
        sens_mult: arguments.value_from_fn("-m", str::parse)?,
        accel: arguments.value_from_fn("-a", str::parse)?,
        cap: arguments
            .value_from_fn("-c", str::parse)
            .unwrap_or(f64::INFINITY),
        offset: arguments.value_from_fn("-o", str::parse).unwrap_or(0.0),
        filename: arguments.free_from_str()?,
    })
}

fn main() -> Result<(), std::io::Error> {
    let mut arguments = Arguments::from_env();
    let help_message = r#"
USAGE: accelerator [OPTIONS] <device-file>

OPTIONS:
  -m SENS_MULTIPLIER    The amount graph of sensitivity is scaled by
  -a ACCELERATION       Slope of sensitivity graph
  -c SENS_CAP           Sets the maximum sensitivity
                        Default: infinity
  -o INPUT_OFFSET       Maximum cursor speed before sensitivity
                        begins increasing
                        Default: 0"#;
    if arguments.contains("-h") {
        println!("{}", help_message);
    }

    let args = match parse_args(&mut arguments) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("Error: {}\n{}", err, help_message);
            std::process::exit(1);
        }
    };

    let file = File::open(args.filename)?;
    let mut source = Device::new_from_file(file)?;
    let out = UInputDevice::create_from_device(&source)?;
    source.grab(GrabMode::Grab)?;

    let mut x = 0.0;
    let mut y = 0.0;
    let mut x_accum = 0.0;
    let mut y_accum = 0.0;
    let mut frame_last_sec = 0;
    let mut frame_last_us = 0;
    let mut sync_flag = ReadFlag::NORMAL; // normal if normal, sync if got SYN_DROPPED
    loop {
        let event = source.next_event(sync_flag | ReadFlag::BLOCKING);
        match event {
            Ok((status, event)) => {
                if status == ReadStatus::Sync {
                    // eat syncs until done (we probably don't need whats in it)
                    eprintln!("Warning: got SYN_DROPPED");
                    sync_flag = ReadFlag::SYNC;
                    continue;
                } else {
                    sync_flag = ReadFlag::NORMAL;
                }

                match event.event_code {
                    EventCode::EV_REL(EV_REL::REL_X) => x = event.value as f64 + x_accum,
                    EventCode::EV_REL(EV_REL::REL_Y) => y = event.value as f64 + y_accum,
                    EventCode::EV_SYN(EV_SYN::SYN_REPORT) => {
                        let change_ms = (event.time.tv_sec as f64 - frame_last_sec as f64) * 1000.0
                            + (event.time.tv_usec as f64 - frame_last_us as f64) / 1000.0;
                        let dist = (x * x + y * y).sqrt();
                        let sensitivity = factor(
                            args.sens_mult,
                            args.accel,
                            args.cap,
                            args.offset,
                            dist / change_ms as f64,
                        );
                        x *= sensitivity;
                        y *= sensitivity;

                        let x_rounded = x.round() as i32;
                        let y_rounded = y.round() as i32;
                        x_accum = x - x_rounded as f64;
                        y_accum = y - y_rounded as f64;

                        out.write_event(&InputEvent {
                            time: event.time,
                            event_code: EventCode::EV_REL(EV_REL::REL_X),
                            value: x_rounded,
                        })?;
                        out.write_event(&InputEvent {
                            time: event.time,
                            event_code: EventCode::EV_REL(EV_REL::REL_Y),
                            value: y_rounded,
                        })?;

                        out.write_event(&event)?;

                        x = 0.0;
                        y = 0.0;
                        frame_last_sec = event.time.tv_sec;
                        frame_last_us = event.time.tv_usec;
                    }
                    _ => out.write_event(&event)?,
                }
            }
            // should never be err? it is blocking
            Err(_) => {
                eprintln!("Error: got back Err from next_event (has the device been closed?)");
                std::process::exit(2);
            }
        }
    }
}
