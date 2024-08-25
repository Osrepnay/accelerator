This directory contains an example udev rule, systemd service, and helper script to automatically start accelerator when the device, the Logitech M570, is added. The helper script (`accelerator-fileconfig`) reads the file located at `/etc/accelerator/<name of device, e.g. logitechrecv>` and passes the arguments to the actual accelerator program.

The file `99-accelerator.rules` should be in `/etc/udev/rules.d`, `accelerator-fileconfig` in `/usr/bin` or `/bin`, `accelerator@.service` in `/etc/systemd/system`, and `logitechrecv` in `/etc/accelerator`.

To adapt this to your mouse, read up on udev rules and update the .rules file and config file accordingly. Rename the config file too; it should match the device name in the udev rule.
