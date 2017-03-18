use libc::{self, c_int, c_ulong};
use uinput_sys::{
    uinput_user_dev,
    EV_KEY,
    EV_ABS,
    EV_MSC,
    BTN_0,
    BTN_TOOL_PEN,
    BTN_TOOL_RUBBER,
    BTN_TOUCH,
    BTN_STYLUS,
    BTN_STYLUS2,
    ABS_PRESSURE,
    ABS_X,
    ABS_Y,
    MSC_SCAN,
};
use std::mem;
use std::slice;
use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;

pub const UI_ABS_SETUP: u64 = 1075598596;
pub const UI_SET_EVBIT: u64 = 1074025828;
pub const UI_SET_KEYBIT: u64 = 1074025829;
pub const UI_SET_ABSBIT: u64 = 1074025831;
pub const UI_SET_MSCBIT: u64 = 1074025832;
pub const UI_DEV_CREATE: u64 = 21761;

#[repr(C)]
struct uinput_abs_setup{
    code: u16,
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

pub unsafe fn create() -> File {
    let mut dev: uinput_user_dev = mem::zeroed();
    let mut file = File::create("/dev/uinput").unwrap();

	set_initial_values(&mut file, &mut dev);
    set_events(&mut file);

    libc::ioctl(file.as_raw_fd(), UI_DEV_CREATE);
    file
}

unsafe fn set_event(fd: c_int, kind: c_ulong, code: c_ulong) {
    assert_ne!(libc::ioctl(fd, kind, code), -1, "Error during set_event");
}

unsafe fn set_events(file: &mut File) {
    let fd = file.as_raw_fd();
    set_event(fd, UI_SET_EVBIT, EV_KEY as u64);
    set_event(fd, UI_SET_EVBIT, EV_ABS as u64);
    set_event(fd, UI_SET_EVBIT, EV_MSC as u64);

    set_event(fd, UI_SET_KEYBIT, BTN_0 as u64);
    set_event(fd, UI_SET_KEYBIT, BTN_TOOL_PEN as u64);
    set_event(fd, UI_SET_KEYBIT, BTN_TOOL_RUBBER as u64);
    set_event(fd, UI_SET_KEYBIT, BTN_TOUCH as u64);
    set_event(fd, UI_SET_KEYBIT, BTN_STYLUS as u64);
    set_event(fd, UI_SET_KEYBIT, BTN_STYLUS2 as u64);

    set_event(fd, UI_SET_ABSBIT, ABS_PRESSURE as u64);
    set_event(fd, UI_SET_ABSBIT, ABS_X as u64);
    set_event(fd, UI_SET_ABSBIT, ABS_Y as u64);

    set_event(fd, UI_SET_MSCBIT, MSC_SCAN as u64);
}

unsafe fn set_initial_values(file: &mut File, dev: &mut uinput_user_dev) {
    let name = "ELAN Pen Fix";
    for (i,b) in name.bytes().enumerate() {
        dev.name[i] = b as i8;
    }
    dev.name[name.len()] = 0;
    dev.id.bustype = 0x18;
    dev.id.vendor = 0x04f3;
    dev.id.product = 0x22ca;
    dev.id.version = 0x100;

    let x = uinput_abs_setup {
        code: ABS_X as u16,
        value: 0,
        minimum: 0,
        maximum: 21464,
        fuzz: 0,
        flat: 0,
        resolution: 62,
    };
    let y = uinput_abs_setup {
        code: ABS_Y as u16,
        value: 0,
        minimum: 0,
        maximum: 12140,
        fuzz: 0,
        flat: 0,
        resolution: 63,
    };

    dev.absmax[ABS_PRESSURE as usize] = 256;
    dev.absmax[ABS_X as usize] = x.maximum;
    dev.absmax[ABS_Y as usize] = y.maximum;

    dev.absmin[ABS_PRESSURE as usize] = 0;
    dev.absmin[ABS_X as usize] = x.minimum;
    dev.absmin[ABS_Y as usize] = y.minimum;

    let xsize = mem::size_of_val(&x);
    let xptr = &x as *const _ as *const u8;
    assert_ne!(libc::ioctl(file.as_raw_fd(), UI_ABS_SETUP, xptr, xsize), -1);
    let ysize = mem::size_of_val(&y);
    let yptr = &y as *const _ as *const u8;
    assert_ne!(libc::ioctl(file.as_raw_fd(), UI_ABS_SETUP, yptr, ysize), -1);

    let size = mem::size_of_val(dev);
    let ptr = dev as *const _ as *const u8;
    let slice = slice::from_raw_parts(ptr, size);
    file.write_all(slice).unwrap();
}
