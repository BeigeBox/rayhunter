# Orbic USB Host Mode

The Orbic RC400L can act as a USB host to connect peripherals (Bluetooth
dongles, Meshtastic radios, serial devices, etc.). The software side works --
`usb_host_enable.ko` switches the OTG controller to host mode and the EHCI
controller enumerates devices. The problem is hardware: the board references
an SMB358 charger IC for VBUS boost power, but the chip isn't populated. The
USB-C port cannot supply power to peripherals.

This page describes a workaround using off-the-shelf parts to tap power from
the Orbic's battery and feed it to peripherals through a Y splitter cable.

## Parts

| Part | Notes |
|------|-------|
| USB-C OTG Y splitter with power input | Must have a USB-C male (to device), USB-A female (to peripheral), and USB-C female (power input). [Example](https://www.amazon.com/dp/B0CXT7RPBC). |
| USB-C male to bare wire pigtail cable | Red/black bare wires on one end, USB-C plug on the other. [Example](https://www.amazon.com/dp/B0BFHXWCS9). |
| Copper foil tape | Available at any hardware store. |
| Resettable PTC fuse, 500mA (optional) | Inline on the positive wire. The Li-ion cell has its own protection circuit, but a fuse is cheap insurance against shorts in your wiring. |

## Hardware

The battery supplies 3.7V nominal (3.0-4.2V range) to VBUS through the Y
splitter. This is below USB spec (5V) but sufficient for low-power peripherals
whose internal regulators accept 3.0V+.

```
                    +------------------+
                    |   Y Splitter     |
Battery  --------→ | USB-C (power in) |
(copper shims +     |                  |
 pigtail cable)     | USB-C (data) ----+--→ Orbic USB-C port
                    |                  |
                    | USB-A (periph) --+--→ BT dongle / Meshtastic / etc.
                    +------------------+
```

### Battery power tap

The Orbic's battery compartment has three spring-pin contacts:

```
[ - ]  [ T ]  [ + ]
left   middle  right
```

The left pin is negative (-), the right pin is positive (+), and the middle
pin is a thermistor for battery temperature monitoring. Only the outer two
pins are needed.

1. Cut two small strips of copper foil tape, roughly the size of the contact
   pads.
2. Trim the pigtail cable to ~15-20cm. Strip the wire ends if needed.
3. If using a polyfuse, put it inline on the red (+) wire.
4. Press the red wire onto one copper tape strip and fold the tape over to
   hold it. Same for black wire on the second strip.
5. Remove the battery. Stick the copper shims onto the positive and negative
   spring contacts (adhesive side down), route the wires out through the
   battery cover seam, and replace the battery. Battery pressure holds
   everything in place. No soldering required.

### Connections

1. Pigtail USB-C plug into the Y splitter's power input port.
2. Y splitter's USB-C male into the Orbic.
3. Peripheral into the Y splitter's USB-A female port.

## Software

USB host mode requires SSH access for full kernel capabilities. ADB's
capability bounding set is too restrictive. See
[Obtaining a shell](./orbic.md#shell) or set up dropbear SSH.

### Kernel modules

The module loading script at `/data/rayhunter/modules/load-modules.sh` runs
at boot. It should load:

```sh
insmod /data/rayhunter/modules/ehci-hcd.ko
insmod /data/rayhunter/modules/ehci-msm.ko
insmod /data/rayhunter/modules/usb_host_enable.ko
insmod /data/rayhunter/modules/cdc-acm.ko       # serial devices
insmod /data/rayhunter/modules/btusb.ko          # bluetooth
```

`usb_host_enable.ko` must load when no USB cable is plugged in. If VBUS is
present (charging cable attached), the mode switch won't take effect.

### Forcing host mode

The module patches the OTG mode but the controller defaults to peripheral.
Force host mode:

```sh
echo 'host' > /sys/kernel/debug/msm_otg/mode
```

Add this to the module loading script to make it persistent across reboots.

### Verifying

```sh
cat /sys/kernel/debug/msm_otg/mode          # should say "host"
ls /sys/bus/usb/devices/                     # look for 1-1:x.x (a real device)
dmesg | grep -i usb | tail                   # look for "new device" messages
ls /dev/ttyACM* /dev/ttyUSB*                 # serial devices
```

## Troubleshooting

**Mode stuck on "peripheral":** VBUS was present when the module loaded.
Unplug USB-C, `rmmod usb_host_enable`, re-insmod, then force host mode.

**No device enumeration:** Peripheral isn't getting power. Check shim
contact, polarity, and that the pigtail is plugged into the Y splitter's
power leg.

**Controller suspends immediately:**
```sh
echo 'on' > /sys/devices/78d9000.usb/power/control
echo 'host' > /sys/kernel/debug/msm_otg/mode
```

## Alternative: powered USB hub

A powered USB hub supplies its own 5V and avoids any hardware modification.
Plug it into the Orbic via an OTG adapter, connect the hub's power supply,
and plug peripherals into the hub.
