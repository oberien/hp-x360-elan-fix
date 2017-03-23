extern crate evdev;
extern crate libc;
extern crate uinput_sys;
extern crate uinput;

mod u_input;

use std::os::unix::io::AsRawFd;
use std::io::Error;

use evdev::Device as EvDevice;
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
    SYN_REPORT,
    MSC_SCAN,
};

const EVIOCGRAB: libc::c_ulong = 1074021776;

fn main() {
    let mut device = None;
    for dev in evdev::enumerate() {
        if dev.name().to_str().unwrap() == "ELAN22CA:00 04F3:22CA Pen" {
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

            if needs_touch && valuex >= 0 && valuey >= 0 {
                input.write(EV_SYN, SYN_REPORT, 0).unwrap();
                input.write(EV_MSC, MSC_SCAN, 0xd0042).unwrap();
                input.write(EV_KEY, BTN_TOUCH, 1).unwrap();
                needs_touch = false;
            }

            if _type == EV_ABS && code == ABS_X {
                valuex = value;
            } else if _type == EV_ABS && code == ABS_Y {
                valuey = value;
            }

            match (_type, code, value) {
                (EV_KEY, BTN_TOOL_RUBBER, _) => {
                    // Map rubber to stylus2
                    input.write(_type, BTN_STYLUS2, value).unwrap();
                },
                (EV_KEY, BTN_TOUCH, 1) => {
                    valuex = -1;
                    valuey = -1;
                    needs_touch = true;
                },
                (EV_MSC, MSC_SCAN, 0xd0042) => {
                    // This is send both on BTN_TOUCH press and release.
                    // We are sending it on press before we send BTN_TOUCH.
                    // In the next match arm we are handling release ones.
                    ()
                },
                (EV_KEY, BTN_TOUCH, 0) => {
                    input.write(EV_MSC, MSC_SCAN, 0xd0042).unwrap();
                    input.write(_type, code, value).unwrap();
                }
                _ => input.write(_type, code, value).unwrap(),
            }
        }
    }
}
