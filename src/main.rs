extern crate evdev;
extern crate libc;
extern crate uinput_sys;
extern crate uinput;

mod u_input;

use evdev::Device as EvDevice;
use uinput::Device as UDevice;
use std::os::unix::io::AsRawFd;

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
    loop {
        let res = libc::epoll_wait(pollfd, &mut evt, 1, -1);
        assert!(res >= 0, "Error waiting for fd: {}", res);
        for evt in device.events_no_sync().unwrap() {
            if evt._type == 1 && evt.code == 321 {
                input.write(evt._type as i32, 332, evt.value).unwrap();
            } else {
                input.write(evt._type as i32, evt.code as i32, evt.value).unwrap();
            }
        }
    }
}
