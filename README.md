# HP x360 ELAN Pen Fix

Libinput [has some bugs](https://gitlab.freedesktop.org/libinput/libinput/issues/381) which affect the ELAN Stylus hardware and software distributed with and used by for example the HP Spectre x360.
This repository provides a userland driver, which implements workarounds around some of the bugs, namely the following ones:

1. Before libinput version 1.15, pressing the eraser button wouldn't work with the ELAN Pen.
   That pen reports one button as Stylus 1, and the other as eraser.
   However, the eraser doesn't get forwarded through libinput.
    * This driver fixes it by mapping the eraser button to Stylus 2, which can be configured separately in most programs (like xournalpp) to support eraser functionality.
2. Since somewhere around libinput 1.13.0, libinput reports the pen to not be in proximity of the screen if it didn't move since ~70ms.
   However, the ELAN Pen has some tolerances built in, which makes it possible to hold still even for multiple seconds.
   This makes working with drawing programs harder, because tools can be deselected just because one didn't move enough.
    * This driver fixes this problem by moving the pointer by 1 pixel left and right every 50ms if no other event has been received and the pen is in proximity.

# Build

To build this project (on linux), make sure that you have the rust compiler `rustc` and `cargo` installed.
If you don't, follow the instructions on [the rust-lang.org installation site](https://www.rust-lang.org/tools/install).

To build and install, run the following commands:

* `make build` will build the userland driver.
* `make install` installs the userland driver and systemd service file
* `systemctl enable hp-x360-elan-fix.service` causes the userland driver to be run on every startup
* `systemctl start hp-x360-elan-fix.service` starts the userland driver without a need to reboot

You can check that the driver is running successfully by checking for the input device `ELAN Pen Fix` in the output of `xinput` (and make sure the service is running with `systemctl status hp-x360-elan-fix.service`).

To uninstall, run `make uninstall`.

