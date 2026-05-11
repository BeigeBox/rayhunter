import type { DeviceInfo, InstallMode } from './types';

export const DEVICES: DeviceInfo[] = [
    {
        id: 'orbic',
        label: 'Orbic RC400L (Network)',
        command: 'orbic',
        fields: [
            {
                key: 'adminPassword',
                label: 'Admin Password',
                type: 'password',
                arg_name: '--admin-password',
            },
            {
                key: 'adminUsername',
                label: 'Admin Username',
                type: 'text',
                default_value: 'admin',
                arg_name: '--admin-username',
                advanced: true,
            },
            {
                key: 'adminIp',
                label: 'Admin IP',
                type: 'text',
                default_value: '192.168.1.1',
                arg_name: '--admin-ip',
                advanced: true,
            },
            {
                key: 'resetConfig',
                label: 'Reset config to defaults',
                type: 'checkbox',
                default_value: false,
                arg_name: '--reset-config',
            },
        ],
        steps: [
            { label: 'Connecting', marker: 'Logging in and starting telnet' },
            { label: 'Rooting', marker: 'Waiting for telnet to become available' },
            { label: 'Transferring files', marker: 'Sending file' },
            { label: 'Rebooting', marker: 'Installation complete. Rebooting' },
            { label: 'Verifying', marker: null },
        ],
    },
    {
        id: 'orbic-usb',
        label: 'Orbic RC400L (USB)',
        command: 'orbic-usb',
        fields: [
            {
                key: 'resetConfig',
                label: 'Reset config to defaults',
                type: 'checkbox',
                default_value: false,
                arg_name: '--reset-config',
            },
        ],
        steps: [
            { label: 'Connecting', marker: 'Forcing a switch into the debug mode' },
            { label: 'Rooting', marker: 'ADB enabled, waiting for reboot' },
            { label: 'Transferring files', marker: 'Installing rootshell' },
            { label: 'Rebooting', marker: 'Waiting for reboot' },
            { label: 'Verifying', marker: 'Testing rayhunter' },
        ],
    },
    {
        id: 'moxee',
        label: 'Moxee Hotspot',
        command: 'moxee',
        fields: [
            {
                key: 'adminPassword',
                label: 'Admin Password',
                type: 'password',
                arg_name: '--admin-password',
            },
            {
                key: 'adminUsername',
                label: 'Admin Username',
                type: 'text',
                default_value: 'admin',
                arg_name: '--admin-username',
                advanced: true,
            },
            {
                key: 'adminIp',
                label: 'Admin IP',
                type: 'text',
                default_value: '192.168.1.1',
                arg_name: '--admin-ip',
                advanced: true,
            },
            {
                key: 'resetConfig',
                label: 'Reset config to defaults',
                type: 'checkbox',
                default_value: false,
                arg_name: '--reset-config',
            },
        ],
        steps: [
            { label: 'Connecting', marker: 'Logging in and starting telnet' },
            { label: 'Rooting', marker: 'Waiting for telnet to become available' },
            { label: 'Transferring files', marker: 'Sending file' },
            { label: 'Rebooting', marker: 'Installation complete. Rebooting' },
            { label: 'Verifying', marker: null },
        ],
    },
    {
        id: 'tmobile',
        label: 'T-Mobile TMOHS1',
        command: 'tmobile',
        fields: [
            {
                key: 'adminIp',
                label: 'Admin IP',
                type: 'text',
                default_value: '192.168.0.1',
                arg_name: '--admin-ip',
            },
            {
                key: 'adminPassword',
                label: 'Admin Password',
                type: 'password',
                arg_name: '--admin-password',
            },
        ],
        steps: [
            { label: 'Connecting', marker: 'Starting telnet' },
            { label: 'Rooting', marker: 'Connecting via telnet to' },
            { label: 'Transferring files', marker: 'Sending file' },
            { label: 'Rebooting', marker: 'Rebooting device and waiting' },
            { label: 'Verifying', marker: 'Testing rayhunter' },
        ],
    },
    {
        id: 'tplink',
        label: 'TP-Link M7350',
        command: 'tplink',
        fields: [
            {
                key: 'adminIp',
                label: 'Admin IP',
                type: 'text',
                default_value: '192.168.0.1',
                arg_name: '--admin-ip',
            },
            {
                key: 'sdcardPath',
                label: 'SD Card Path',
                type: 'text',
                default_value: '',
                arg_name: '--sdcard-path',
                advanced: true,
            },
            {
                key: 'skipSdcard',
                label: 'Skip SD card (use internal storage)',
                type: 'checkbox',
                default_value: false,
                arg_name: '--skip-sdcard',
                advanced: true,
            },
            {
                key: 'resetConfig',
                label: 'Reset config to defaults',
                type: 'checkbox',
                default_value: false,
                arg_name: '--reset-config',
            },
        ],
        steps: [
            { label: 'Connecting', marker: 'Launching telnet on the device' },
            { label: 'Rooting', marker: 'Connecting via telnet to' },
            { label: 'Transferring files', marker: 'Sending file' },
            { label: 'Rebooting', marker: 'Done. Rebooting device' },
            { label: 'Verifying', marker: null },
        ],
    },
    {
        id: 'wingtech',
        label: 'Wingtech CT2MHS01',
        command: 'wingtech',
        fields: [
            {
                key: 'adminIp',
                label: 'Admin IP',
                type: 'text',
                default_value: '192.168.1.1',
                arg_name: '--admin-ip',
            },
            {
                key: 'adminPassword',
                label: 'Admin Password',
                type: 'password',
                arg_name: '--admin-password',
            },
        ],
        steps: [
            { label: 'Connecting', marker: 'Starting telnet' },
            { label: 'Rooting', marker: 'Connecting via telnet to' },
            { label: 'Transferring files', marker: 'Sending file' },
            { label: 'Rebooting', marker: 'Rebooting device and waiting' },
            { label: 'Verifying', marker: 'Testing rayhunter' },
        ],
    },
    {
        id: 'uz801',
        label: 'UZ801',
        command: 'uz801',
        fields: [
            {
                key: 'adminIp',
                label: 'Admin IP',
                type: 'text',
                default_value: '192.168.100.1',
                arg_name: '--admin-ip',
            },
        ],
        steps: [
            { label: 'Connecting', marker: 'Activating USB debugging backdoor' },
            { label: 'Rooting', marker: 'Waiting for device reboot and ADB' },
            { label: 'Transferring files', marker: 'Installing rayhunter files' },
            { label: 'Rebooting', marker: 'Rebooting the device' },
            { label: 'Verifying', marker: 'Installation complete!' },
        ],
    },
    {
        id: 'pinephone',
        label: 'PinePhone',
        command: 'pinephone',
        fields: [],
        steps: [
            { label: 'Connecting', marker: 'Unlocking modem' },
            { label: 'Transferring files', marker: 'Sending file' },
            { label: 'Rebooting', marker: 'Rebooting device and waiting' },
            { label: 'Verifying', marker: 'Testing rayhunter' },
        ],
    },
];

// Restricts a field to specific install modes. Keyed by `<device_id>.<field_key>`.
// Stream 4 reads this when rendering ConfigForm and constructing args; fields
// without an entry apply to every mode.
export const FIELD_APPLIES_TO: Record<string, ReadonlyArray<InstallMode>> = {
    'orbic.resetConfig': ['install'],
    'orbic-usb.resetConfig': ['install'],
    'moxee.resetConfig': ['install'],
    'tplink.resetConfig': ['install'],
};
