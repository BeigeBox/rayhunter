import type { ErrorGuidance } from './types';

interface ErrorPattern {
    match: string;
    title: string;
    message: string;
}

const PATTERNS: ErrorPattern[] = [
    // Connection / reachability
    {
        match: 'Failed to get login info',
        title: "Can't reach the device",
        message: 'Check your WiFi connection and make sure you are connected to the hotspot.',
    },
    {
        match: 'Failed to reach device at',
        title: "Can't reach the device",
        message: 'Check your WiFi connection and make sure you are connected to the hotspot.',
    },
    {
        match: 'Failed to connect to shell',
        title: "Can't reach the device",
        message: 'Check your connection and make sure the device is powered on.',
    },
    {
        match: 'Failed to install rayhunter on the',
        title: 'Installation failed',
        message: 'Check your connection to the device and retry.',
    },

    // Authentication
    {
        match: 'Login failed: incorrect password',
        title: 'Wrong password',
        message: 'Check the sticker on your device for the correct password.',
    },
    {
        match: 'Login failed with retcode:',
        title: 'Login failed',
        message: 'Double-check your credentials.',
    },
    {
        match: 'login did not return a token in response',
        title: 'Login failed',
        message: 'Double-check your password.',
    },
    {
        match: '--admin-password is required',
        title: 'Password required',
        message: 'This device requires a password to install.',
    },

    // USB device
    {
        match: 'No Orbic device found',
        title: 'Device not detected',
        message: 'Try a different USB cable or port.',
    },
    {
        match: 'being used by another program',
        title: 'Device is busy',
        message: 'Another program is using the device. Close other USB tools and retry.',
    },
    {
        match: 'Permission denied',
        title: 'Device is busy',
        message: 'Another program is using the device. Close other USB tools (like adb) and retry.',
    },
    {
        match: 'Timeout waiting for ADB connection',
        title: 'Device not responding',
        message: 'Device not responding after USB debug activation. Retry.',
    },
    {
        match: 'ADB connection error',
        title: 'USB connection error',
        message: 'Try a different USB cable or port.',
    },

    // Rooting / telnet
    {
        match: 'Timeout waiting for telnet to become available',
        title: 'Rooting took too long',
        message: 'Power-cycle the device and retry.',
    },
    {
        match: 'rootshell is not giving us root',
        title: 'Rooting failed',
        message: 'Power-cycle the device and retry.',
    },
    {
        match: 'Bad result code when trying to root device',
        title: 'Rooting failed',
        message: 'Try a different hardware revision exploit.',
    },

    // File transfer
    {
        match: 'File transfer unsuccessful',
        title: 'File transfer corrupted',
        message: 'Retry the installation.',
    },
    {
        match: 'MD5 verification failed',
        title: 'File transfer corrupted',
        message: 'Retry the installation.',
    },
    {
        match: 'not found in:',
        title: 'Command failed on device',
        message: 'Copy the log and report an issue.',
    },

    // TP-Link SD card
    {
        match: 'Unable to determine sdcard path',
        title: 'No SD card found',
        message: 'Insert a FAT-formatted SD card and retry.',
    },
    {
        match: 'FAT-formatted SD card',
        title: 'SD card mount failed',
        message: 'Make sure the SD card is FAT-formatted.',
    },
];

export function classify_error(error_text: string): ErrorGuidance {
    for (const pattern of PATTERNS) {
        if (error_text.includes(pattern.match)) {
            return { title: pattern.title, message: pattern.message };
        }
    }
    return {
        title: 'Something went wrong',
        message: 'Copy the log and report an issue.',
    };
}
