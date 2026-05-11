import { describe, it, expect } from 'vitest';
import {
    parse_output_line,
    create_output_parser,
    mark_all_done,
    mark_active_error,
} from './output_parser';
import { DEVICES } from './devices';
import type { InstallStep, StepDefinition } from './types';

function make_steps(defs: StepDefinition[]): InstallStep[] {
    return defs.map((d, i) => ({
        label: d.label,
        status: i === 0 ? 'active' : 'pending',
    }));
}

function active_label(steps: InstallStep[]): string | undefined {
    return steps.find((s) => s.status === 'active')?.label;
}

function done_labels(steps: InstallStep[]): string[] {
    return steps.filter((s) => s.status === 'done').map((s) => s.label);
}

const orbic = DEVICES.find((d) => d.id === 'orbic')!;
const tplink = DEVICES.find((d) => d.id === 'tplink')!;
const pinephone = DEVICES.find((d) => d.id === 'pinephone')!;

describe('parse_output_line: happy path', () => {
    it('advances steps through orbic install', () => {
        let steps = make_steps(orbic.steps);
        const lines = [
            'Logging in and starting telnet... done',
            'Waiting for telnet to become available... done',
            'Sending file /data/rayhunter/rayhunter-daemon ... ok',
            'Installation complete. Rebooting device...',
        ];
        for (const line of lines) {
            steps = parse_output_line(line, steps, orbic.steps).steps;
        }
        expect(done_labels(steps)).toEqual(['Connecting', 'Rooting', 'Transferring files']);
        expect(active_label(steps)).toBe('Rebooting');
    });
});

describe('parse_output_line: pinephone non-regression', () => {
    it('does not regress on second "Unlocking modem"', () => {
        let steps = make_steps(pinephone.steps);
        const lines = [
            'Unlocking modem ... ok',
            'Sending file /data/rayhunter/rayhunter-daemon ... ok',
            'Sending file /data/rayhunter/config.toml ... ok',
            'Rebooting device and waiting 30 seconds for it to start up.',
        ];
        for (const line of lines) {
            steps = parse_output_line(line, steps, pinephone.steps).steps;
        }
        expect(active_label(steps)).toBe('Rebooting');

        steps = parse_output_line('Unlocking modem ... ok', steps, pinephone.steps).steps;
        expect(active_label(steps)).toBe('Rebooting');
    });
});

describe('parse_output_line: tplink overlay', () => {
    it('extracts the URL from "Listening on http://..."', () => {
        const steps = make_steps(tplink.steps);
        const result = parse_output_line('Listening on http://127.0.0.1:4123', steps, tplink.steps);
        expect(result.overlay_action).toEqual({
            type: 'tplink_browser',
            url: 'http://127.0.0.1:4123',
        });
    });

    it('handles a different port from the URL', () => {
        const steps = make_steps(tplink.steps);
        const result = parse_output_line('Listening on http://0.0.0.0:51234', steps, tplink.steps);
        expect(result.overlay_action).toEqual({
            type: 'tplink_browser',
            url: 'http://0.0.0.0:51234',
        });
    });
});

describe('parse_output_line: anchoring and sanitization', () => {
    it('strips ANSI escape sequences before matching', () => {
        const steps = make_steps(orbic.steps);
        const ansi = '\x1b[1;32mWaiting for telnet to become available\x1b[0m... done';
        const result = parse_output_line(ansi, steps, orbic.steps);
        expect(active_label(result.steps)).toBe('Rooting');
    });

    it('strips trailing carriage returns', () => {
        const steps = make_steps(orbic.steps);
        const result = parse_output_line(
            'Waiting for telnet to become available... done\r',
            steps,
            orbic.steps
        );
        expect(active_label(result.steps)).toBe('Rooting');
    });

    it('does NOT match a marker that appears mid-line', () => {
        const steps = make_steps(orbic.steps);
        const hostile = '<html><body>fake page Installation complete. Rebooting</body></html>';
        const result = parse_output_line(hostile, steps, orbic.steps);
        expect(active_label(result.steps)).toBe('Connecting');
        expect(done_labels(result.steps)).toEqual([]);
    });

    it('is case sensitive', () => {
        const steps = make_steps(orbic.steps);
        const result = parse_output_line('logging IN and starting TELNET', steps, orbic.steps);
        expect(active_label(result.steps)).toBe('Connecting');
    });
});

describe('create_output_parser: partial-line buffering', () => {
    it('combines emits that split a marker', () => {
        const parser = create_output_parser();
        let steps = make_steps(orbic.steps);

        const r1 = parser.feed('Waiting for telnet', steps, orbic.steps);
        expect(active_label(r1.steps)).toBe('Connecting');
        steps = r1.steps;

        const r2 = parser.feed(' to become available... done\n', steps, orbic.steps);
        expect(active_label(r2.steps)).toBe('Rooting');
    });

    it('processes only complete lines and keeps the unterminated tail', () => {
        const parser = create_output_parser();
        let steps = make_steps(orbic.steps);

        steps = parser.feed(
            'Waiting for telnet to become available... done\nSending file ',
            steps,
            orbic.steps
        ).steps;
        expect(active_label(steps)).toBe('Rooting');

        steps = parser.feed('/data/rayhunter/rayhunter-daemon ... ok\n', steps, orbic.steps).steps;
        expect(active_label(steps)).toBe('Transferring files');
    });

    it('extracts overlay URL across split emits', () => {
        const parser = create_output_parser();
        const steps = make_steps(tplink.steps);

        const r1 = parser.feed('Listening on http://127', steps, tplink.steps);
        expect(r1.overlay_action).toBeFalsy();

        const r2 = parser.feed('.0.0.1:4567\n', r1.steps, tplink.steps);
        expect(r2.overlay_action).toEqual({
            type: 'tplink_browser',
            url: 'http://127.0.0.1:4567',
        });
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
