extern crate evdev;
extern crate libc;
extern crate uinput_sys;
extern crate uinput;

mod u_input;

use std::os::unix::io::AsRawFd;
use std::io::Error;

use evdev::{Device as EvDevice, Types};
use uinput::Device as UDevice;
use uinput_sys::{
    EV_SYN,
    EV_KEY,
    EV_ABS,
    EV_MSC,
    BTN_TOOL_RUBBER,
    BTN_TOUCH,
    BTN_STYLUS2,
    ABS_X,
    ABS_Y,
    ABS_PRESSURE,
    SYN_REPORT,
    MSC_SCAN,
};

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
    let pollfd = libc::epoll_create(1);
    assert!(pollfd >= 0, "Error creating epoll: {}: {:?}", pollfd, Error::last_os_error());
    let mut evt = libc::epoll_event {
        events: libc::EPOLLIN as u32,
        u64: device.fd() as u64,
    };
    let res = libc::epoll_ctl(pollfd, libc::EPOLL_CTL_ADD, device.fd(), &mut evt);
    assert!(res >= 0, "Error adding fd to poll: {}: {:?}", res, Error::last_os_error());
    let mut valuex = -1;
    let mut valuey = -1;
    let mut pressure = -1;
    let mut needs_touch = false;
    loop {
        let res = libc::epoll_wait(pollfd, &mut evt, 1, -1);
        if res == -1 && Error::last_os_error().raw_os_error().unwrap() == 4 {
            // EINTR is gotten on suspends
            continue;
        }
        assert!(res >= 0, "Error waiting for fd: {}: {:?}", res, Error::last_os_error());
        for evt in device.events_no_sync().unwrap() {
            let _type = evt._type as i32;
            let code = evt.code as i32;
            let value = evt.value;

            if needs_touch && valuex >= 0 && valuey >= 0 && pressure >= 0 {
                // For some reason some layer between the event device and
                // the actual applications, a lot of events are just swallowed.
                // Therefore we just generate 5 extra events in hope that one
                // will survive the event cannibalism.
                for (dx, dy) in (-5..0).zip(-5..0) {
                    input.write(EV_SYN, SYN_REPORT, 0).unwrap();
                    input.write(EV_ABS, ABS_X, valuex + dx).unwrap();
                    input.write(EV_ABS, ABS_Y, valuey + dy).unwrap();
                }
                input.write(EV_SYN, SYN_REPORT, 0).unwrap();
                input.write(EV_MSC, MSC_SCAN, 0xd0042).unwrap();
                input.write(EV_KEY, BTN_TOUCH, 1).unwrap();
                input.write(EV_ABS, ABS_PRESSURE, pressure).unwrap();
                needs_touch = false;
            }

            match (_type, code, value) {
                (EV_KEY, BTN_TOOL_RUBBER, _) => {
                    // Map rubber to stylus2
                    input.write(_type, BTN_STYLUS2, value).unwrap();
                    //input.write(EV_MSC, MSC_SCAN, 0xd003c).unwrap();
                    //input.write(_type, BTN_TOOL_RUBBER, value).unwrap();
                },
                (EV_KEY, BTN_TOUCH, 1) => {
                    valuex = -1;
                    valuey = -1;
                    pressure = -1;
                    needs_touch = true;
                },
                (EV_MSC, MSC_SCAN, 0xd0042) => {
                    // This is send both on BTN_TOUCH press and release.
                    // We are sending it on press before we send BTN_TOUCH.
                    // In the next match arm we are handling release ones.
                    ()
                },
                (EV_ABS, ABS_PRESSURE, val) if needs_touch => pressure = val,
                (EV_ABS, ABS_X, x) if needs_touch => valuex = x,
                (EV_ABS, ABS_Y, y) if needs_touch => valuey = y,
                (EV_KEY, BTN_TOUCH, 0) => {
                    input.write(EV_MSC, MSC_SCAN, 0xd0042).unwrap();
                    input.write(_type, code, value).unwrap();
                }
                _ => input.write(_type, code, value).unwrap(),
            }
        }
    }
}
