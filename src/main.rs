extern crate evdev;
extern crate libc;
extern crate uinput_sys;
extern crate uinput;
extern crate mio;

mod u_input;

use std::os::unix::io::AsRawFd;
use std::io::Error;
use std::time::Duration;

use evdev::{Device as EvDevice};
use uinput::Device as UDevice;
#[allow(unused_imports)]
use uinput_sys::{
    EV_SYN,
    EV_KEY,
    EV_ABS,
    EV_MSC,
    BTN_TOOL_PEN,
    BTN_TOOL_RUBBER,
    BTN_TOUCH,
    BTN_STYLUS,
    BTN_STYLUS2,
    ABS_X,
    ABS_Y,
    ABS_PRESSURE,
    SYN_REPORT,
    MSC_SCAN,
};
use mio::{unix::EventedFd, Events, Poll, Ready, PollOpt, Token};

const EVIOCGRAB: libc::c_ulong = 1074021776;

fn main() {
    let expected_types = evdev::SYNCHRONIZATION
        | evdev::KEY
        | evdev::ABSOLUTE
        | evdev::MISC;
    let mut device = None;
    for dev in evdev::enumerate() {
        if dev.name().to_str().unwrap() == "ELAN22CA:00 04F3:22CA"
            && dev.input_id().vendor == 0x4f3
            && dev.input_id().product == 0x22ca
            && dev.input_id().version == 0x100
            && dev.events_supported().contains(expected_types)
            && dev.keys_supported().contains(320)
            && dev.keys_supported().contains(321)
            && dev.keys_supported().contains(330)
            && dev.keys_supported().contains(331)
        {
            device = Some(dev);
            break;
        }
    }
    let mut device = device.unwrap();

    let res = unsafe { libc::ioctl(device.fd(), EVIOCGRAB, 1) };
    assert!(res >= 0, "Error grabbing event device: {}: {:?}", res, Error::last_os_error());

    let file = unsafe { u_input::create() };
    let mut input = UDevice::new(file.as_raw_fd());

    unsafe { main_loop(&mut device, &mut input) }
}

unsafe fn main_loop(device: &mut EvDevice, input: &mut UDevice) -> ! {
    let poll = Poll::new().expect("can't create Poll");
    let mut events = Events::with_capacity(1024);

    // register polls
    poll.register(&EventedFd(&device.fd()), Token(0), Ready::readable(), PollOpt::edge()).unwrap();

    let mut x = 0;
    let mut is_in_proximity = false;
    loop {
        // we need to send events every ~45ms due to the proximity out quirk of libinput
        // https://gitlab.freedesktop.org/libinput/libinput/issues/381
        let timeout = if is_in_proximity {
            Some(Duration::from_millis(45))
        } else {
            None
        };
        poll.poll(&mut events, timeout).expect("poll failed");
        if events.is_empty() {
            // timeout fired, send slightly modified x event to prevent proximity quirk
            if x % 2 == 0 {
                x += 1;
            } else {
                x -= 1;
            }
            input.write(EV_ABS, ABS_X, x).unwrap();
            input.write(EV_SYN, SYN_REPORT, 0).unwrap();
        }
        for poll_evt in &events {
            if poll_evt.token() == Token(0) {
                for evt in device.events_no_sync().unwrap() {
                    let typ = evt._type as i32;
                    let code = evt.code as i32;
                    let value = evt.value;

                    match (typ, code, value) {
                        (EV_KEY, BTN_TOOL_RUBBER, _) => {
                            // Map rubber to stylus2
                            // https://gitlab.freedesktop.org/libinput/libinput/issues/259
                            input.write(typ, BTN_STYLUS2, value).unwrap();
                        }
                        (EV_ABS, ABS_X, _) => {
                            x = value;
                            input.write(EV_ABS, ABS_X, value).unwrap();
                        }
                        (EV_KEY, BTN_TOOL_PEN, 0) => {
                            is_in_proximity = false;
                            input.write(EV_KEY, BTN_TOOL_PEN, 0).unwrap();
                        }
                        (EV_KEY, BTN_TOOL_PEN, 1) => {
                            is_in_proximity = true;
                            input.write(EV_KEY, BTN_TOOL_PEN, 1).unwrap();
                        }
                        _ => input.write(typ, code, value).unwrap(),
                    }
                }
            }
        }
    }
}
