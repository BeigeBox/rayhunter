import { describe, it, expect } from 'vitest';
import { parse_output_line, mark_all_done, mark_active_error } from './output_parser';
import { DEVICES } from './devices';
import type { InstallStep, StepDefinition } from './types';

function make_steps(defs: StepDefinition[]): InstallStep[] {
    return defs.map((d, i) => ({
        label: d.label,
        status: i === 0 ? 'active' : 'pending',
    }));
}

function feed_lines(
    lines: string[],
    defs: StepDefinition[]
): { steps: InstallStep[]; overlays: ('tplink-login' | null)[] } {
    let steps = make_steps(defs);
    const overlays: ('tplink-login' | null)[] = [];
    for (const line of lines) {
        const result = parse_output_line(line, steps, defs);
        steps = result.steps;
        if (result.overlay) overlays.push(result.overlay);
    }
    return { steps, overlays };
}

function active_label(steps: InstallStep[]): string | undefined {
    return steps.find((s) => s.status === 'active')?.label;
}

function done_labels(steps: InstallStep[]): string[] {
    return steps.filter((s) => s.status === 'done').map((s) => s.label);
}

const orbic = DEVICES.find((d) => d.id === 'orbic')!;
const orbicUsb = DEVICES.find((d) => d.id === 'orbic-usb')!;
const tmobile = DEVICES.find((d) => d.id === 'tmobile')!;
const tplink = DEVICES.find((d) => d.id === 'tplink')!;
const wingtech = DEVICES.find((d) => d.id === 'wingtech')!;
const uz801 = DEVICES.find((d) => d.id === 'uz801')!;
const pinephone = DEVICES.find((d) => d.id === 'pinephone')!;

describe('output_parser: orbic network', () => {
    const lines = [
        'Logging in and starting telnet... done',
        'Waiting for telnet to become available... done',
        'Sending file /data/rayhunter/rayhunter-daemon ... ok',
        'Sending file /data/rayhunter/config.toml ... ok',
        'Sending file /etc/init.d/rayhunter_daemon ... ok',
        'Installation complete. Rebooting device...',
    ];

    it('advances through output-triggered steps', () => {
        const { steps } = feed_lines(lines, orbic.steps);
        // Rebooting is the last step with a marker, so it's active
        // Verifying has marker=null, stays pending (GUI triggers it)
        expect(done_labels(steps)).toEqual(['Connecting', 'Rooting', 'Transferring files']);
        expect(active_label(steps)).toBe('Rebooting');
    });

    it('stays on Connecting before second marker', () => {
        const { steps } = feed_lines(lines.slice(0, 1), orbic.steps);
        expect(active_label(steps)).toBe('Connecting');
    });

    it('mark_all_done completes everything after output', () => {
        const { steps } = feed_lines(lines, orbic.steps);
        const final = mark_all_done(steps);
        expect(final.every((s) => s.status === 'done')).toBe(true);
    });
});

describe('output_parser: orbic usb', () => {
    const lines = [
        'Forcing a switch into the debug mode to enable ADB',
        "ADB enabled, waiting for reboot... it's alive!",
        'Waiting for atfwd_daemon to startup... done',
        'Installing rootshell... done',
        'Installing rayhunter... done',
        'Waiting for reboot... done',
        'Testing rayhunter... done',
    ];

    it('advances through all steps', () => {
        const { steps } = feed_lines(lines, orbicUsb.steps);
        expect(done_labels(steps)).toEqual([
            'Connecting',
            'Rooting',
            'Transferring files',
            'Rebooting',
        ]);
        expect(active_label(steps)).toBe('Verifying');
    });
});

describe('output_parser: tmobile', () => {
    const lines = [
        'Starting telnet ... ok',
        'Connecting via telnet to 192.168.0.1 ... ok',
        'Sending file /data/rayhunter/config.toml ... ok',
        'Rebooting device and waiting 30 seconds for it to start up.',
        'Testing rayhunter ... ok',
    ];

    it('advances through all steps', () => {
        const { steps } = feed_lines(lines, tmobile.steps);
        expect(done_labels(steps)).toEqual([
            'Connecting',
            'Rooting',
            'Transferring files',
            'Rebooting',
        ]);
        expect(active_label(steps)).toBe('Verifying');
    });
});

describe('output_parser: tplink', () => {
    it('handles v3 path', () => {
        const lines = [
            'Launching telnet on the device',
            'Detected hardware revision v3, successfully opened telnet',
            'Connecting via telnet to 192.168.0.1',
            'Mounting sdcard on /media/card',
            'Sending file /media/card/config.toml ... ok',
            'Done. Rebooting device.',
        ];
        const { steps, overlays } = feed_lines(lines, tplink.steps);
        // Verifying has marker=null so Rebooting is last active
        expect(done_labels(steps)).toEqual(['Connecting', 'Rooting', 'Transferring files']);
        expect(active_label(steps)).toBe('Rebooting');
        expect(overlays).toHaveLength(0);
    });

    it('handles v5 path with overlay trigger', () => {
        const lines = [
            'Launching telnet on the device',
            "This doesn't look like a v3 device, trying web-based exploit",
            'Listening on http://127.0.0.1:4000',
            'Please open above URL in your browser and log into the router to continue.',
            'Connecting via telnet to 192.168.0.1',
        ];
        const { steps, overlays } = feed_lines(lines, tplink.steps);
        expect(overlays).toContain('tplink-login');
        expect(done_labels(steps)).toContain('Connecting');
        expect(active_label(steps)).toBe('Rooting');
    });

    it('Mounting sdcard triggers transfer step', () => {
        const lines = [
            'Launching telnet on the device',
            'Connecting via telnet to 192.168.0.1',
            'Mounting sdcard on /media/card',
            'Sending file /media/card/rayhunter-daemon ... ok',
        ];
        const { steps } = feed_lines(lines, tplink.steps);
        expect(done_labels(steps)).toContain('Rooting');
        expect(active_label(steps)).toBe('Transferring files');
    });
});

describe('output_parser: wingtech', () => {
    const lines = [
        'Starting telnet ... ok',
        'Connecting via telnet to 192.168.1.1 ... ok',
        'Sending file /data/rayhunter/config.toml ... ok',
        'Rebooting device and waiting 30 seconds for it to start up.',
        'Testing rayhunter ... ok',
    ];

    it('advances through all steps', () => {
        const { steps } = feed_lines(lines, wingtech.steps);
        expect(done_labels(steps)).toEqual([
            'Connecting',
            'Rooting',
            'Transferring files',
            'Rebooting',
        ]);
        expect(active_label(steps)).toBe('Verifying');
    });
});

describe('output_parser: uz801', () => {
    const lines = [
        'Activating USB debugging backdoor... Checking if device is online... ok',
        'ok',
        'Waiting for device reboot and ADB connection... ok',
        'Installing rayhunter files... ok',
        'Modifying startup script... ok',
        'Rebooting the device... ok',
        'Installation complete!',
    ];

    it('advances through all steps', () => {
        const { steps } = feed_lines(lines, uz801.steps);
        expect(done_labels(steps)).toEqual([
            'Connecting',
            'Rooting',
            'Transferring files',
            'Rebooting',
        ]);
        expect(active_label(steps)).toBe('Verifying');
    });
});

describe('output_parser: pinephone', () => {
    const lines = [
        'Unlocking modem ... ok',
        'Sending file /data/rayhunter/rayhunter-daemon ... ok',
        'Sending file /data/rayhunter/config.toml ... ok',
        'Rebooting device and waiting 30 seconds for it to start up.',
        'Unlocking modem ... ok',
        'Testing rayhunter ... ok',
    ];

    it('advances through all steps', () => {
        const { steps } = feed_lines(lines, pinephone.steps);
        expect(done_labels(steps)).toEqual(['Connecting', 'Transferring files', 'Rebooting']);
        expect(active_label(steps)).toBe('Verifying');
    });

    it('does not regress on second "Unlocking modem"', () => {
        const partial = lines.slice(0, 4);
        const { steps: before } = feed_lines(partial, pinephone.steps);
        expect(active_label(before)).toBe('Rebooting');

        const { steps: after } = feed_lines(
            [...partial, 'Unlocking modem ... ok'],
            pinephone.steps
        );
        expect(active_label(after)).toBe('Rebooting');
    });
});

describe('mark_all_done', () => {
    it('marks all steps as done', () => {
        const steps = make_steps(orbic.steps);
        const result = mark_all_done(steps);
        expect(result.every((s) => s.status === 'done')).toBe(true);
    });
});

describe('mark_active_error', () => {
    it('marks the active step as error', () => {
        const steps: InstallStep[] = [
            { label: 'A', status: 'done' },
            { label: 'B', status: 'active' },
            { label: 'C', status: 'pending' },
        ];
        const result = mark_active_error(steps);
        expect(result[0].status).toBe('done');
        expect(result[1].status).toBe('error');
        expect(result[2].status).toBe('pending');
    });
});
