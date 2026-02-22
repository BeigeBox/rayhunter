# Orbic Device Emulator - Implementation Plan

## Goal

QEMU-based emulator that boots a faithful reproduction of the Orbic RC400L
environment with a mock `/dev/diag` kernel module. Developers can run the
installer, daemon, and web UI against it without a real device.

## Mock /dev/diag Fidelity Requirements

Traced through the daemon's DIAG init sequence (`lib/src/diag_device.rs`,
`daemon/src/diag.rs`). The mock device must handle this exact flow:

### Startup Sequence

1. **open()** - `/dev/diag` opened with `O_RDWR`
2. **ioctl(7)** - `DIAG_IOCTL_SWITCH_LOGGING` with mode=2 (`MEMORY_DEVICE_MODE`).
   Daemon tries simple form first, then retries with `DiagLoggingModeParam` struct
   if that fails. Mock should accept both, return 0.
3. **ioctl(32)** - `DIAG_IOCTL_REMOTE_DEV`. Daemon passes pointer to `int use_mdm = 0`.
   Mock should leave it at 0 (meaning: include 4-byte mdm_field in requests).
4. **write()** - `RetrieveIdRanges` request. Binary format:
   - 4-byte DataType (32 = UserSpace, little-endian)
   - 4-byte mdm_field (-1 as i32, since use_mdm=0)
   - HDLC-encapsulated payload: opcode=115, subopcode=1
5. **read()** - Mock must return a `MessagesContainer` with the response:
   - `data_type: 32` (UserSpace)
   - `num_messages: 1`
   - HDLC-encapsulated response: opcode=115, subopcode=1, status=0,
     followed by 16 x u32 `log_mask_sizes`. Set entry 11 (LTE) to a
     nonzero bitsize (e.g. 4096) to enable LTE RRC logging.
6. **write()** - `SetMask` request for each log type with bitsize > 0.
   Binary: opcode=115, subopcode=3, log_type, bitsize, mask bytes.
   The mask has bits set for specific log codes (0xb0c0, 0xb0e2, etc).
7. **read()** - Mock returns `SetMask` response with status=0 for each.
8. **Continuous read()** - After init, daemon polls read() in a loop.
   Mock returns `MessagesContainer` structs with HDLC-encapsulated
   `Message::Log` entries from the scenario QMDL file.

### Binary Framing

**MessagesContainer** (what read() returns):
```
[u32 data_type=32] [u32 num_messages=N]
  [u32 msg_len] [msg_bytes...]   // repeated N times
```

Each message is HDLC-encapsulated (escape 0x7e/0x7d, CRC-CCITT, 0x7e terminator).

**RequestContainer** (what write() receives):
```
[u32 data_type=32] [i32 mdm_field=-1] [hdlc_payload...]
```

### Recommendation

Implement the full handshake in the kernel module. It's ~50 extra lines of C
on top of the ring-buffer replay, and it catches init bugs that a "just start
streaming" approach would miss. The daemon expects specific response formats
and will error out if the handshake fails.

Pre-compute the canned responses (RetrieveIdRanges, SetMask) as static byte
arrays in the module. On open(), reset state machine to expect handshake.
After handshake completes, switch to streaming from the scenario QMDL file.

## Display Stubbing

The daemon already has a headless display driver (`daemon/src/display/headless.rs`)
used by the Pinephone target. For the emulator, configure `device = "pinephone"`
in config.toml to get headless display. No code changes needed.

Later, when the device-UI feature lands (writing to the Orbic's 128x128 LCD via
framebuffer), the emulator will need a virtual framebuffer that can be viewed
from the host (e.g. QEMU's `-display` option or VNC). This is a Phase 5 concern.

## Partition Layout & Size Constraints

The Orbic uses NAND flash (256 MB) with UBI. What matters for emulation:

### What the daemon cares about

- **`statvfs` on QMDL store path** (`/data/rayhunter/qmdl`): The daemon calls
  `libc::statvfs` (in `daemon/src/stats.rs`) to check free space. It uses
  `min_space_to_start_recording_mb` and `min_space_to_continue_recording_mb`
  (both default 1 MB) to decide whether to start/continue recording. It
  checks every 256 KB of data written (`DISK_CHECK_BYTES_INTERVAL`).
- **Root filesystem mount flags**: The network installer (`installer/src/orbic_network.rs:203`)
  runs `mount -o remount,rw /dev/ubi0_0 /` before writing to `/etc/init.d/`.
  Root is normally read-only on the real device.
- **Separate `/data` partition**: The installer writes to both `/` (init scripts,
  rootshell) and `/data/rayhunter/` (daemon binary, config). These must be
  on separate mount points to match real behavior.

### Real device partition map (from /proc/mtd, df, mounts)

All of rayhunter's filesystems live on a single MTD partition:

    mtd16: 0x16d80000 (383 MB) "system" — UBI device 0
      ubi0:rootfs   -> /          67 MB total, 54 MB used  (ubifs, rw on dev device, ro on fresh)
      ubi0:usrfs    -> /data     220 MB total, 16 MB used  (ubifs, rw)
      ubi0:cachefs  -> /cache     32 MB total, ~0 used     (ubifs, rw)

Other MTD partitions exist (sbl, mibib, efs2, tz, rpm, aboot, boot, modem,
misc, recovery, recoveryfs, sec) but none are relevant to rayhunter.
`mtd13` ("usrdata", 25 MB) mounts at `/usrdata` via a separate UBI device
and is NOT where rayhunter stores data.

### Emulated partition layout

Single 512 MB partitioned SD image (`disk.img`) with MBR partition table:

| Partition | Mount | Size | Flags | Contents |
|-----------|-------|------|-------|----------|
| `mmcblk0p1` | `/` (root) | 128 MB | Mounted **ro** by kernel | BusyBox, init scripts, /bin, /etc, /lib |
| `mmcblk0p2` | `/data` | 383 MB | Mounted **rw** by init | rayhunter/, mock-diag/, qmdl/ |

QEMU vexpress boards only have one SD slot, so two separate images won't work.
The 512 MB total is required because QEMU requires power-of-2 SD card sizes.
ext4 images must be created with `-O ^metadata_csum,^64bit` for 3.18 compat.

Root is mounted read-only to match a fresh device. The init scripts (or the
installer when testing) must `mount -o remount,rw /` before writing to it,
just like on the real Orbic.

### Permissions model (from real device dump)

| Path | Owner | Mode | Notes |
|------|-------|------|-------|
| `/bin/` | root | 755 | Standard BusyBox layout |
| `/etc/init.d/` | root | 755 | SysV init scripts |
| `/bin/rootshell` | root | 4755 (suid) | Installed by installer, gives uid 0 |
| `/data/rayhunter/` | root | 755 | Created by installer |
| `/data/rayhunter/rayhunter-daemon` | root | 755 | Installed by installer |
| `/dev/diag` | root:diag | 0660 | Major 244, minor 0. Group `diag` (gid 53) has rw |

Users/groups (from actual `/etc/passwd` and `/etc/group`):
- uid 0: root (has DES password hash in passwd, not shadowed)
- uid 53: diag (member of groups: sdcard, rebooters)
- No uid 1000/2000 — this is NOT Android, it's a stripped BusyBox userland

Key daemons (from `/etc/inittab`):
- `mbimd` — modem interface daemon (runlevel 5, respawn)
- `diagrebootapp` — diag reboot handler (runlevel 5, respawn)
- `fs-scrub-daemon` — filesystem scrubber (runlevel 2-5, respawn)
- Serial console on `ttyHSL0` at 115200

The daemon runs as root (started by init). The installer writes files as
root via rootshell (ADB path) or telnet (network path).

## Rootfs Construction

### Step 1: Dump from real device

SSH into the Orbic and use `tar` to capture the live mounted filesystems.
The tarballs stream directly to the host (nothing written on-device).

**Root cause analysis of failed dump approaches:**

1. **`/dev/mtdblockN` reads hang — UBI contention.**
   `mtdblock` provides a linear-offset block device abstraction designed for
   NOR flash. It has zero bad block awareness and doesn't know about UBI's
   logical-to-physical block remapping. When UBI is attached to the same MTD
   partition (which it is for mtd16/system), both hold concurrent references
   to the same flash hardware — the kernel does not enforce mutual exclusion.
   UBI actively performs wear leveling and garbage collection, remapping
   physical blocks while mtdblock tries to read linearly. The hang occurs
   when the Qualcomm QPIC NAND controller's hardware mutex serializes access
   and mtdblock's read blocks behind an ongoing UBI erase/write cycle.
   mtd0 (sbl) worked because it's the bootloader — a raw partition not
   managed by UBI, no concurrent erase activity. Linux 5.15+ added a kernel
   warning for this exact scenario: "MTD device is NAND, please consider
   using UBI block devices instead." The Orbic runs 3.18 and has no warning.

2. **`nanddump` stalls over SSH — Dropbear's 24 KiB receive window.**
   Dropbear's default SSH receive window is 24,576 bytes. Only 24 KiB of data
   can be in-flight before the sender waits for acknowledgment. With the
   Orbic's ~300ms WiFi round-trip latency, theoretical max throughput is
   ~80 KB/s. A 2.5 MB partition would take ~30 seconds (we killed after 60s
   assuming it was hung). Combined with 64 KiB pipe buffer backpressure,
   `nanddump` spends most of its time sleeping in `pipe_write`. The fix would
   be to increase Dropbear's window (compile-time `RECV_MAX_PAYLOAD_LEN` or
   client-side `-W` flag), or dump to a file on-device first.

3. **BusyBox tar v1.23.2 lacks `--one-file-system`.**
   Must manually `--exclude` each mount point directory instead. The
   `--exclude` flag works without `./` prefix.

4. **tar over SSH works because gzip compresses the data** (84 MB -> 38 MB)
   and filesystem reads bypass the NAND controller contention issues since
   they go through UBI/UBIFS properly.

5. **Stalled reads caused device-wide hangs.** Blocked mtdblock reads left
   kernel threads in the NAND driver's wait queue. Multiple stale SSH
   sessions exhausted Dropbear's connection limit, and ADB shell commands
   queued behind blocked processes. Required a full reboot to clear.

6. **UBI volume character devices readable via ADB.** `dd if=/dev/ubi0_N`
   reads UBI volumes at the logical level (above wear leveling) without
   needing `ubiblock`. `ubiblock -c` fails with EBUSY on mounted volumes,
   but the character device reads work fine concurrently with mounted UBIFS.
   ADB `exec-out` bypasses SSH entirely, achieving ~1.9 MB/s over USB
   (6x faster than SSH over WiFi). All five UBI volumes across three UBI
   devices dumped in ~4 minutes total.

**What was captured:**

```
scratch/emulator/reference/
  rootfs.tar.gz      38 MB   Root filesystem (/) excluding mount points
  data.tar.gz         7 MB   /data (rayhunter config, binaries, QMDLs)
  firmware.tar.gz    27 MB   /firmware (modem firmware)
  mtd/
    mtd0_sbl.img    2.5 MB   Raw bootloader (not UBI-managed)
    ubi0_0_rootfs.img 74 MB  UBI volume: rootfs (logical, no UBI metadata)
    ubi0_1_usrfs.img 232 MB  UBI volume: usrfs/data
    ubi0_2_cachefs.img 37 MB UBI volume: cachefs
    ubi1_0_modem.img  43 MB  UBI volume: modem firmware
    ubi3_0_usrdata.img 13 MB UBI volume: usrdata
  kernel_version.txt          Linux 3.18.48 armv7l
  mtd.txt                     /proc/mtd (17 partitions)
  mounts.txt                  /proc/mounts
  df.txt                      Disk usage
  ubinfo.txt                  UBI volume info
  passwd.txt                  /etc/passwd
  group.txt                   /etc/group
  diag_stat.txt               /dev/diag permissions
  inittab.txt                 /etc/inittab
```

**What's missing vs a full forensic copy:**
- Raw NAND for mtd1-mtd15 (mibib, efs2, tz, rpm, aboot, boot, modem,
  scrub, misc, recovery, recoveryfs, sec — ~130 MB total)
- Raw NAND for mtd16/system (383 MB) — we have the logical UBI volume
  contents but not the physical layer (erase counters, bad block markers,
  OOB/ECC data, wear leveling tables)
- The missing partitions contain bootchain firmware, modem NV items, and
  secure boot keys. Not needed for rayhunter emulation, only for full
  SoC boot emulation or device forensics.

**Future raw NAND dump options (if ever needed):**
- `nanddump -f /tmp/file.img /dev/mtdN` — dump to tmpfs (78 MB), then
  transfer. Works for partitions under ~70 MB.
- Increase Dropbear receive window for streaming large partitions over SSH.
- For mtd16 (383 MB), would need chunked approach or direct ADB with
  `nanddump` piped through `adb exec-out`.

**Dump script:** See `emulator/scripts/dump-real-device.sh`.

This reference directory is NOT checked into the repo. The dump contains
dev device artifacts (custom configs, test QMDLs, wifi client binaries,
dropbear SSH keys, etc) that need to be cleaned before converting to
emulator images.

### Step 2: Convert to QEMU-bootable images

See `emulator/scripts/build-images.sh` (host wrapper) and
`emulator/scripts/build-images-inner.sh` (Docker inner script).

Creates a single 512 MB partitioned SD image with MBR table (p1=128MB rootfs,
p2=383MB data). Uses Alpine Docker with e2fsprogs. ext4 created with
`-O ^metadata_csum,^64bit` for kernel 3.18 compatibility.

The script handles artifact cleanup: removes dropbear keys, SSH keys, wifi
creds, QMDLs, crash logs. Patches inittab (`ttyHSL0` -> `ttyAMA0`). Creates
missing mount point directories. Resets config.toml from template.

### Step 3: Reconstruct for reproducibility (Buildroot)

The dd-based images are great for quick emulation but aren't reproducible
(they contain device-specific state). For CI and distribution, build a
clean rootfs with Buildroot that matches the dump's structure:

- BusyBox 1.x (static, armv7) providing `/bin/sh` and standard utils
- `/etc/init.d/` with SysV init scripts (rayhunter_daemon, mock-diag)
- adbd (static armv7 build) or netcat listener for installer testing
- User/group setup matching the real device (root uid 0, diag uid/gid 53)
- `/dev/diag` owned root:diag with mode 0660
- `/bin/rootshell` as suid stub that execs its args as root
- Root image mounted read-only by default
- Data image (~128 MB) mounted rw at `/data`
- `/dev/diag` created by mock_diag.ko at boot

Buildroot defconfig targeting `BR2_arm` + `BR2_cortex_a7` with Bootlin
external toolchain (`BR2_TOOLCHAIN_EXTERNAL_BOOTLIN_ARMV7_EABIHF_MUSL_STABLE`).
See Kernel section for toolchain details.

Verify against the dd-based images:
- Compare `find -ls` output for critical paths
- Verify uid/gid ownership matches on /bin, /etc/init.d
- Verify partition sizes and mount flags match

## Kernel

Use kernel 3.18.x to match the real device (runs 3.18.48). Using the same
kernel version catches syscall compatibility issues, ioctl behavior differences,
and /proc/sys layout mismatches that a modern kernel would silently paper over.
The DIAG ioctl numbers and char device semantics haven't changed, but the
overall system environment (cgroups, tmpfs defaults, /dev population) differs
enough between 3.18 and 6.x to matter for faithful emulation.

Building 3.18 requires an older GCC (5.x-7.x) — modern GCC 13+ fails on
removed headers and changed semantics. Buildroot handles this via its
"external toolchain" feature, using a pre-built Bootlin toolchain.

**Bootlin toolchains:** Pre-built GCC cross-compilers from Bootlin, a
reputable French embedded Linux company (est. 2004). They're a top 15 Linux
kernel contributor and their CTO (Thomas Petazzoni) co-maintains Buildroot
itself. The toolchains are built with Buildroot's own infrastructure, fully
open source (github.com/bootlin/toolchains-builder), hosted at
toolchains.bootlin.com. For 3.18 on ARMv7, use the `armv7-eabihf` toolchain
with GCC 5.4 or 7.x and musl libc.

Buildroot config:
- `BR2_TOOLCHAIN_EXTERNAL=y`
- `BR2_TOOLCHAIN_EXTERNAL_BOOTLIN=y`
- `BR2_TOOLCHAIN_EXTERNAL_BOOTLIN_ARMV7_EABIHF_MUSL_STABLE=y`
- `BR2_LINUX_KERNEL_CUSTOM_VERSION_VALUE="3.18.48"`

Target: vexpress-a15 machine (Cortex-A15, supports SDIV/UDIV needed by Orbic
glibc). Cannot use vexpress-a9 because its Cortex-A9 lacks SDIV/UDIV and the
board locks the CPU model (no `-cpu` override allowed).

Kernel built from `vexpress_defconfig` with these additions enabled via
`scripts/config --enable`:
- `EXT4_FS`, `DEVTMPFS`, `DEVTMPFS_MOUNT`
- `MODULES`, `MODULE_UNLOAD` (for mock_diag.ko)
- `UNIX`, `INET`, `NET`, `PACKET`
- `MMC`, `MMC_ARMMMCI` (SD card support)
- `SMSC911X` (built-in NIC for network access)

**Known missing (suspected):** `CONFIG_VFP`, `CONFIG_NEON`, `CONFIG_VFPv3` —
these may not be in vexpress_defconfig and would explain the SIGILL on Orbic
binaries that use VFPv3+NEON. This is the current investigation.

## Mock /dev/diag Kernel Module

~150 lines of C. Implements:

- Character device at `/dev/diag` with major number auto-allocated
- `open()`: Load scenario file into ring buffer, reset state to handshake phase
- `read()`: In handshake phase, return canned responses. After handshake,
  return chunks from scenario QMDL wrapped in MessagesContainer framing.
- `write()`: Parse enough to identify request type (RetrieveIdRanges vs SetMask),
  advance state machine. Log commands for debugging.
- `ioctl()`: Accept SWITCH_LOGGING (7) and REMOTE_DEV (32), return 0.
- Module parameter: `scenario_path` (default `/data/mock-diag/scenario.qmdl`)

## Boot Script & Orchestration

See `emulator/orbic-emulator.sh` (host wrapper) and `emulator/scripts/boot-inner.sh`
(Docker inner script). The QEMU command is documented in the "Current Status"
section above.

Notes on QEMU machine choice:
- **vexpress-a15** with Cortex-A15. Cannot use vexpress-a9 because its
  Cortex-A9 lacks SDIV/UDIV and the board locks the CPU model.
- Root is mounted `ro` to match a fresh device. Init scripts or the installer
  must `mount -o remount,rw /` before writing to rootfs.
- vexpress boards have a **built-in** SMSC LAN9118 NIC. It is NOT a pluggable
  device — use `-net nic` not `-device lan9118`.
- The real device root is `/dev/ubi0_0`; the emulator uses `/dev/mmcblk0p1`.
  Phase 3 (installer testing) will need to handle this mismatch.
- QEMU runs inside Docker (debian:bullseye) to avoid requiring local QEMU
  installation on macOS.

Init sequence inside VM:
1. Mount `/data` from second SD image
2. `insmod /lib/modules/mock_diag.ko scenario_path=/data/mock-diag/scenario.qmdl`
3. Start adbd on TCP 5555
4. Start rayhunter-daemon via `/etc/init.d/rayhunter_daemon`

From host:
- `adb connect localhost:5555`
- `http://localhost:8080` for web UI
- `./installer orbic` to test installer against VM

## Scenario QMDL Files

Assume these exist (capture/sanitize workflow is TBD):
- `scenarios/normal_camping.qmdl` + `.expected.json` - no alerts
- `scenarios/imsi_request.qmdl` + `.expected.json` - triggers identity_request heuristic
- `scenarios/2g_downgrade.qmdl` + `.expected.json` - triggers 2G downgrade heuristic
- `scenarios/null_cipher.qmdl` + `.expected.json` - triggers null cipher heuristic

## File Structure

```
emulator/
  Makefile                    # Build kernel, rootfs, mock module
  orbic-emulator.sh           # Launch script
  scripts/
    ci-run-scenario.sh
    ci-test-installer.sh
    dump-real-device.sh       # Capture rootfs from real Orbic
  kernel/
    .config                   # vexpress defconfig for chosen kernel
    patches/                  # If any needed
  rootfs/
    buildroot-defconfig
    overlay/                  # Files overlaid onto rootfs
      etc/init.d/
        rayhunter_daemon
        mock-diag
      etc/passwd
      bin/rootshell           # Suid stub
    build-rootfs.sh
  mock-diag/
    mock_diag.c               # Kernel module source
    Makefile                  # Builds against kernel headers
  scenarios/                  # QMDL files + expected results
```

## Current Status & Session Log

### What's Been Built

All scripts are in `emulator/scripts/` (host wrappers) or `emulator/` (launch script).
Large artifacts are excluded from git via `.git/info/exclude`:
`emulator/reference/`, `emulator/vm/`, `emulator/build/`.

**Scripts created:**

| Script | Purpose | Status |
|--------|---------|--------|
| `emulator/scripts/dump-real-device.sh` | SSH/ADB dump from real Orbic | Done, tested |
| `emulator/scripts/build-kernel.sh` | Host wrapper: Docker kernel build | Done, tested |
| `emulator/scripts/build-kernel-inner.sh` | Docker inner: cross-compile 3.18.48 | Done, tested |
| `emulator/scripts/build-images.sh` | Host wrapper: Docker disk image build | Done, tested |
| `emulator/scripts/build-images-inner.sh` | Docker inner: partitioned disk image | Done, tested |
| `emulator/scripts/boot-inner.sh` | Docker inner: QEMU boot | Done, needs fixes |
| `emulator/orbic-emulator.sh` | Main launch script | Done, needs fixes |

**Built artifacts (in `emulator/vm/`, not tracked):**

| File | Size | Description |
|------|------|-------------|
| `zImage` | 3.3 MB | Linux 3.18.48 for vexpress (vexpress_defconfig + extras) |
| `vexpress-v2p-ca15-tc1.dtb` | 14 KB | Device tree for vexpress-a15 |
| `vexpress-v2p-ca9.dtb` | 14 KB | Device tree for vexpress-a9 (not currently used) |
| `disk.img` | 512 MB | Partitioned SD image (p1=128MB rootfs, p2=383MB data) |

**Cached build artifacts (in `emulator/build/`, not tracked):**

| Directory | Description |
|-----------|-------------|
| `armv7-eabihf--musl--stable-2018.11-1/` | Bootlin toolchain (GCC 7.3.0, musl) |
| `linux-3.18.48/` | Kernel source with built objects |

**Reference data (in `emulator/reference/`, not tracked):**
- `rootfs.tar.gz` (38 MB) — Root filesystem dump from real Orbic
- `data.tar.gz` (7 MB) — /data partition dump
- `config.toml.in` — Clean config template

### Key Decisions & Fixes Applied

1. **Kernel 3.18.48 with Bootlin toolchain** — matches real device kernel version.
   Bootlin toolchain: `armv7-eabihf--musl--stable-2018.11-1` (GCC 7.3.0).
   Kernel headers are 4.9 in the toolchain but this is a non-issue with musl.

2. **`--platform linux/amd64` required** for all Docker builds on Apple Silicon.
   The Bootlin toolchain is x86_64 Linux binaries.

3. **`-fcommon` GCC patch** — kernel 3.18's DTC has a `yylloc` multiple definition
   bug with GCC 10+ (which defaults to `-fno-common`). Fixed by patching the
   kernel Makefile to append `-fcommon` to HOSTCFLAGS.

4. **ext4 features `^metadata_csum,^64bit`** — modern mkfs.ext4 enables features
   that kernel 3.18 doesn't support. Both must be disabled. The `64bit` feature
   was the harder one to find (error code 0x2000 in mount).

5. **Single partitioned disk image** — vexpress only has one SD card slot.
   512 MB total (power-of-2 required by QEMU). MBR partition table:
   p1=128MB rootfs (mounted ro), p2=383MB data (mounted rw by init).
   Built with sfdisk + dd-at-offset (losetup doesn't work in Docker).

6. **vexpress-a15 board** — switched from vexpress-a9 because Cortex-A9 lacks
   SDIV/UDIV. However, switching to A15 alone didn't fix the SIGILL (see below).

7. **Rootfs cleanup** — build-images-inner.sh removes: dropbear keys, SSH
   authorized_keys, wpa_supplicant.conf, wifi-creds.conf, QMDLs, crash logs.
   Patches inittab: `ttyHSL0` -> `ttyAMA0`. Creates missing mount point dirs:
   /dev, /proc, /sys, /data, /cache, /run, /firmware, /usrdata.

8. **NIC config** — vexpress boards have a built-in SMSC LAN9118. It's NOT a
   pluggable device (`-device lan9118` fails). Use `-net nic -net user,...`
   to connect the built-in NIC.

### Current Blocker: Orbic Binaries SIGILL

**Symptom:** Every dynamically linked Orbic binary (busybox, init, everything)
crashes with `Illegal instruction` immediately after the kernel hands off to
userspace. This happens on both vexpress-a9 (Cortex-A9) and vexpress-a15
(Cortex-A15).

**What works:**
- Kernel boots, mounts ext4, devtmpfs works, all subsystems initialize
- A statically linked BusyBox (downloaded from busybox.net, musl/soft-float)
  executes successfully as init — proves kernel + QEMU work
- QEMU **user-mode emulation** (`qemu-arm -L /mnt/rootfs /mnt/rootfs/bin/busybox`)
  runs ALL Orbic binaries perfectly on cortex-a7, cortex-a15, and max CPU models

**What fails:**
- Full system emulation: any Orbic dynamically linked binary = SIGILL
- The SIGILL happens during dynamic linker startup (ld-2.22.so / glibc 2.22)

**Binary analysis of Orbic rootfs:**
```
/sbin/init -> /sbin/init.sysvinit (symlink, resolves inside rootfs)
/bin/busybox: ELF 32-bit LSB shared, ARM EABI5, dynamically linked, ld-linux.so.3
  Tag_CPU_arch: v7, Tag_FP_arch: VFPv3, Tag_Advanced_SIMD_arch: NEONv1
  NEEDED: libc.so.6, ld-linux.so.3
  SDIV/UDIV instructions: 0

/lib/ld-linux.so.3 -> ld-2.22.so (glibc 2.22 dynamic linker)
/lib/libgcc_s.so.1: 47 SDIV/UDIV instructions (but A15 supports these)
```

**Root cause hypothesis:**
Since `qemu-arm` user-mode works but full system emulation doesn't, the issue
is between the kernel and userspace — NOT the binary instructions themselves.
Most likely: the 3.18 kernel isn't properly enabling VFP/NEON coprocessor
access for userspace on the QEMU vexpress-a15 board. The static BusyBox that
works is soft-float (no VFP instructions). The Orbic glibc uses VFPv3+NEON.

Alternative hypotheses:
- Kernel HWCAP reporting differs from what glibc ifunc resolvers expect
- The vexpress_defconfig doesn't enable CONFIG_VFP or CONFIG_NEON
- glibc 2.22's startup code uses a Qualcomm-specific instruction

**Diagnostic approach (not yet completed):**
1. Boot with static BusyBox as init, mount /proc, check /proc/cpuinfo to see
   what features the kernel reports. Compare to real device.
2. Check kernel .config for CONFIG_VFP, CONFIG_NEON, CONFIG_VFPv3.
3. If VFP/NEON is missing from kernel config, add it and rebuild.
4. If VFP is enabled but still fails, disassemble the exact crash point by
   checking kernel dmesg for the fault address.

**Fallback approach if kernel VFP fix doesn't work:**
Replace the Orbic's glibc (ld-2.22.so, libc.so.6, libgcc_s.so.1) with the
Bootlin toolchain's musl libc. The Orbic applications only use standard
POSIX/Linux APIs and don't need Qualcomm-specific glibc optimizations. This
is a clean fix since the rayhunter-daemon binary is already compiled with musl.

### Reproducing the Current State

From a clean checkout on `device-emulator` branch:

```sh
# 1. Get reference data from real device (need Orbic connected via SSH/ADB)
./emulator/scripts/dump-real-device.sh

# 2. Build kernel (first run downloads toolchain + source, ~10 min)
./emulator/scripts/build-kernel.sh

# 3. Build disk image (needs reference tarballs from step 1)
./emulator/scripts/build-images.sh

# 4. Boot (currently hits SIGILL on Orbic init)
./emulator/orbic-emulator.sh
```

### QEMU Command (current, in boot-inner.sh)

```sh
qemu-system-arm \
    -M vexpress-a15 -m 512M \
    -kernel /vm/zImage \
    -dtb /vm/vexpress-v2p-ca15-tc1.dtb \
    -append "root=/dev/mmcblk0p1 console=ttyAMA0 rootfstype=ext4 ro" \
    -drive file=/vm/disk-run.img,format=raw,if=sd \
    -net nic -net user,hostfwd=tcp::8080-:8080,hostfwd=tcp::5555-:5555 \
    -nographic \
    -no-reboot
```

## Implementation Phases

### Phase 1: Bootable Orbic VM
- Dump rootfs from real device for reference
- Set up Buildroot defconfig, build minimal rootfs
- Cross-compile kernel for vexpress-a9
- Boot in QEMU, get shell, verify filesystem matches Orbic layout
- Set up permission model (root owns everything, diag group gid 53 for /dev/diag)
- Get adbd running so `adb connect localhost:5555` works
- **Deliverable:** `orbic-emulator.sh` boots to an Orbic-like shell

### Phase 2: Mock /dev/diag Module
- Write and compile mock_diag.ko
- Implement full handshake state machine (ioctls, RetrieveIdRanges, SetMask)
- Implement scenario QMDL replay after handshake
- Load at boot, verify `/dev/diag` exists with correct permissions
- Test: daemon starts without errors, processes scenario data
- **Deliverable:** Rayhunter boots and processes a QMDL scenario in the VM

### Phase 3: Installer Testing
- Build a "clean" rootfs variant (no Rayhunter pre-installed)
- Run the real installer against the VM via ADB
- Verify it installs rootshell, config, daemon, init scripts
- **Deliverable:** `./installer orbic` works against the VM

### Phase 4: CI Integration
- Pre-build VM images (kernel + rootfs + data), store as release artifacts
- GitHub Actions workflow: build Rayhunter, inject into VM, run scenarios
- 3-4 scenario tests
- Installer smoke test
- **Deliverable:** PR checks include device emulator tests

### Phase 5: Developer Experience (ongoing)
- `make emulator` in the repo root
- Hot-reload via virtio-9p mount
- Virtual framebuffer for device-UI feature testing
- Snapshot/restore for quick iteration
- Add TP-Link M7350 as second VM target
