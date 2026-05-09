import type { InstallerState } from './state.svelte';
import type { DeviceInfo, ErrorGuidance } from './types';
import { DEVICES } from './devices';

export type MockOutcome =
    | { kind: 'success-verified' }
    | { kind: 'success-unverified' }
    | { kind: 'failure'; error: ErrorGuidance };

export const FAILURE_SCENARIOS: { label: string; error: ErrorGuidance }[] = [
    {
        label: "Can't reach device",
        error: {
            title: "Can't reach the device",
            message: 'Check your WiFi connection and make sure you are connected to the hotspot.',
        },
    },
    {
        label: 'Wrong password',
        error: {
            title: 'Wrong password',
            message: 'Check the sticker on your device for the correct password.',
        },
    },
    {
        label: 'USB not detected',
        error: {
            title: 'Device not detected',
            message: 'Try a different USB cable or port.',
        },
    },
    {
        label: 'Rooting failed',
        error: {
            title: 'Rooting failed',
            message: 'Power-cycle the device and retry.',
        },
    },
    {
        label: 'File transfer corrupted',
        error: {
            title: 'File transfer corrupted',
            message: 'Retry the installation.',
        },
    },
    {
        label: 'No SD card',
        error: {
            title: 'No SD card found',
            message: 'Insert a FAT-formatted SD card and retry.',
        },
    },
    {
        label: 'Generic error',
        error: {
            title: 'Something went wrong',
            message: 'Copy the log and report an issue.',
        },
    },
];

const FAKE_LOG_LINES = [
    'Checking device compatibility...',
    'Preparing installation files...',
    'Establishing connection to device...',
    'Authenticating with device admin panel...',
    'Connection established successfully.',
    'Starting root process...',
    'Mounting filesystem read-write...',
    'Creating /data/rayhunter directory...',
];

export function jump_to(
    installer: InstallerState,
    target: string,
    device?: DeviceInfo,
    failureIndex?: number
) {
    const dev = device ?? DEVICES[0];
    installer.reset();

    if (target === 'device-select') return;

    installer.select_device(dev);
    if (target === 'config') return;

    installer.start_install(dev, 'install');

    if (target === 'progress') {
        installer.append_output('Connecting to device...\n');
        if (dev.steps.length > 1 && dev.steps[1].marker) {
            installer.append_output(dev.steps[1].marker + '\n');
        }
        return;
    }

    if (target === 'success-verified') {
        installer.install_succeeded(dev, true);
        return;
    }
    if (target === 'success-unverified') {
        installer.install_succeeded(dev, false);
        return;
    }
    if (target.startsWith('failure')) {
        const scenario = FAILURE_SCENARIOS[failureIndex ?? FAILURE_SCENARIOS.length - 1];
        installer.install_failed(dev, scenario.error);
        return;
    }
    if (target === 'tplink-overlay') {
        installer.append_output('Listening on http://127.0.0.1:4000\n');
        return;
    }
}

export function mock_run(
    installer: InstallerState,
    device: DeviceInfo,
    outcome: MockOutcome,
    fail_at_step?: number
): () => void {
    installer.reset();
    installer.select_device(device);
    installer.start_install(device, 'install');

    const markers = device.steps.map((s) => s.marker).filter((m): m is string => m !== null);
    const total_steps = markers.length;
    const fail_step = fail_at_step ?? Math.max(0, total_steps - 1);
    let step = 0;
    let cancelled = false;

    const timer = setInterval(() => {
        if (cancelled) return;

        if (step < total_steps) {
            if (outcome.kind === 'failure' && step === fail_step) {
                installer.append_output(`Error: mock failure triggered\n`);
                installer.install_failed(device, outcome.error);
                clearInterval(timer);
                return;
            }
            const fake_line = FAKE_LOG_LINES[step % FAKE_LOG_LINES.length];
            installer.append_output(fake_line + '\n');
            installer.append_output(markers[step] + '\n');
            step++;
        } else {
            clearInterval(timer);
            if (outcome.kind === 'success-verified') {
                installer.install_succeeded(device, true);
            } else if (outcome.kind === 'success-unverified') {
                installer.install_succeeded(device, false);
            }
        }
    }, 800);

    return () => {
        cancelled = true;
        clearInterval(timer);
    };
}
