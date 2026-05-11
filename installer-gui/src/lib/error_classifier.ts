import type { ErrorGuidance } from './types';

interface ErrorPattern {
    match: string;
    title: string;
    message: string;
}

const PATTERNS: ErrorPattern[] = [
    {
        match: 'Installation cancelled.',
        title: 'Installation cancelled',
        message: 'No changes were made to the device.',
    },

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

    {
        match: 'Login failed: incorrect password',
        title: 'Wrong password',
        message:
            'The admin password did not work. See https://efforg.github.io/rayhunter/ for the device-specific setup guide.',
    },
    {
        match: 'Login failed with retcode:',
        title: 'Login failed',
        message:
            'The device rejected the login. Double-check the password. The retcode is the device firmware status code from the login attempt.',
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
        match: 'Timeout waiting for ADB connection',
        title: 'Device not responding',
        message: 'Device not responding after USB debug activation. Retry.',
    },
    {
        match: 'ADB connection error',
        title: 'USB connection error',
        message:
            'On Windows, check the device driver in Device Manager. On Linux or macOS, unplug the device and plug it back in, then retry.',
    },

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
        message:
            'This device may be a different hardware revision than expected. Try selecting a different exploit option in advanced settings, or check the device-specific docs at https://efforg.github.io/rayhunter/.',
    },

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
        match: 'Unable to determine sdcard path',
        title: 'No SD card found',
        message:
            'Insert a FAT32-formatted SD card into the router. If your device does not have an SD card slot, enable "Skip SD card" in advanced settings.',
    },
    {
        match: 'FAT-formatted SD card',
        title: 'SD card not usable',
        message:
            'Rayhunter needs a FAT32-formatted SD card. Reformat the card as FAT32 and retry, or enable "Skip SD card" in advanced settings to use internal storage.',
    },
];

const USB_HINT_RE = /(LIBUSB_ERROR_ACCESS|libusb|\busb\b)/i;

function strip_secrets(text: string): string {
    return text
        .replace(/--admin-password\s+\S+/gi, '--admin-password ***')
        .replace(/--password\s+\S+/gi, '--password ***')
        .replace(/--token\s+\S+/gi, '--token ***');
}

export function classify_error(error_text: string): ErrorGuidance {
    const safe = strip_secrets(error_text);

    if (safe.includes('Permission denied') && USB_HINT_RE.test(safe)) {
        return {
            title: 'USB device is busy',
            message:
                'macOS or another program is holding the USB device. Close other USB tools (like adb) and retry. If you have adb installed, try `adb kill-server`.',
        };
    }

    for (const pattern of PATTERNS) {
        if (safe.includes(pattern.match)) {
            return { title: pattern.title, message: pattern.message };
        }
    }
    return {
        title: 'Something went wrong',
        message:
            'Use the Copy Log and Report Issue buttons below to share details with the maintainers.',
    };
}
