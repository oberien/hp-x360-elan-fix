extern crate evdev;
extern crate libc;
extern crate uinput_sys;
extern crate uinput;

mod u_input;

use std::os::unix::io::AsRawFd;

use evdev::Device as EvDevice;
use uinput::Device as UDevice;
use uinput_sys::{
    EV_KEY,
    EV_ABS,
    BTN_TOOL_RUBBER,
    BTN_TOUCH,
    BTN_STYLUS2,
    ABS_X,
    ABS_Y,
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
    assert!(res >= 0, "Error grabbing event device: {}", res);

    let file = unsafe { u_input::create() };
    let mut input = UDevice::new(file.as_raw_fd());

    unsafe { main_loop(&mut device, &mut input) }
}

unsafe fn main_loop(device: &mut EvDevice, input: &mut UDevice) -> ! {
    let pollfd = libc::epoll_create(1);
    assert!(pollfd >= 0, "Error creating epoll: {}", pollfd);
    let mut evt = libc::epoll_event {
        events: libc::EPOLLIN as u32,
        u64: device.fd() as u64,
    };
    let res = libc::epoll_ctl(pollfd, libc::EPOLL_CTL_ADD, device.fd(), &mut evt);
    assert!(res >= 0, "Error adding fd to poll: {}", res);
    let mut x_changed = false;
    let mut y_changed = false;
    let mut needs_touch = false;
    loop {
        let res = libc::epoll_wait(pollfd, &mut evt, 1, -1);
        assert!(res >= 0, "Error waiting for fd: {}", res);
        for evt in device.events_no_sync().unwrap() {
            let _type = evt._type as i32;
            let code = evt.code as i32;
            let value = evt.value;

            if _type == EV_ABS && code == ABS_X {
                x_changed = true;
            } else if _type == EV_ABS && code == ABS_Y {
                y_changed = true;
            }

            if needs_touch && x_changed && y_changed {
                input.write(EV_KEY, BTN_TOUCH, 1).unwrap();
            }

            if _type == EV_KEY && code == BTN_TOOL_RUBBER {
                input.write(_type, BTN_STYLUS2, value).unwrap();
            } else if _type == EV_KEY && code == BTN_TOUCH && value == 1 {
                x_changed = false;
                y_changed = false;
                needs_touch = true;
            } else {
                input.write(_type, code, value).unwrap();
            }
        }
    }
}
