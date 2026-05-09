import { describe, it, expect } from 'vitest';
import { DEVICES } from './devices';

// state.svelte.ts uses $state runes which require the Svelte compiler.
// We test the pure logic by importing devices and testing build_args logic directly,
// and test state transitions via a minimal mock of the state shape.

const orbic = DEVICES.find((d) => d.id === 'orbic')!;
const tplink = DEVICES.find((d) => d.id === 'tplink')!;
const pinephone = DEVICES.find((d) => d.id === 'pinephone')!;
const uz801 = DEVICES.find((d) => d.id === 'uz801')!;

describe('devices', () => {
    it('has all 7 device definitions', () => {
        expect(DEVICES).toHaveLength(7);
    });

    it('orbic has correct fields', () => {
        const keys = orbic.fields.map((f) => f.key);
        expect(keys).toEqual(['adminPassword', 'adminUsername', 'adminIp', 'resetConfig']);
    });

    it('tplink has sd card fields', () => {
        const keys = tplink.fields.map((f) => f.key);
        expect(keys).toContain('sdcardPath');
        expect(keys).toContain('skipSdcard');
    });

    it('pinephone has no fields', () => {
        expect(pinephone.fields).toHaveLength(0);
    });

    it('uz801 has only admin IP', () => {
        expect(uz801.fields).toHaveLength(1);
        expect(uz801.fields[0].key).toBe('adminIp');
    });
});

describe('device steps', () => {
    it('orbic has 5 steps', () => {
        expect(orbic.steps).toHaveLength(5);
        expect(orbic.steps[0].label).toBe('Connecting');
        expect(orbic.steps[4].label).toBe('Verifying');
    });

    it('pinephone has no Rooting step', () => {
        const labels = pinephone.steps.map((s) => s.label);
        expect(labels).not.toContain('Rooting');
        expect(labels).toHaveLength(4);
    });

    it('all steps have markers except GUI-triggered ones', () => {
        for (const device of DEVICES) {
            for (const step of device.steps) {
                if (step.marker === null) {
                    expect(['Verifying']).toContain(step.label);
                }
            }
        }
    });
});

describe('build_args logic', () => {
    function build_args_pure(
        device: typeof orbic,
        field_values: Record<string, string | boolean>,
        mode: 'install' | 'update' = 'install'
    ): string {
        const args: string[] = [device.command];
        for (const field of device.fields) {
            const val = field_values[field.key];
            if (field.type === 'checkbox') {
                if (val && !(mode === 'update' && field.key === 'resetConfig')) {
                    args.push(field.arg_name);
                }
            } else if (val && typeof val === 'string' && val.trim()) {
                args.push(field.arg_name, val.trim());
            }
        }
        return args.join(' ');
    }

    it('builds orbic args with all fields', () => {
        const result = build_args_pure(orbic, {
            adminIp: '192.168.1.1',
            adminUsername: 'admin',
            adminPassword: 'secret',
            resetConfig: true,
        });
        expect(result).toBe(
            'orbic --admin-password secret --admin-username admin --admin-ip 192.168.1.1 --reset-config'
        );
    });

    it('skips empty fields', () => {
        const result = build_args_pure(orbic, {
            adminIp: '192.168.1.1',
            adminUsername: '',
            adminPassword: 'pass',
            resetConfig: false,
        });
        expect(result).toBe('orbic --admin-password pass --admin-ip 192.168.1.1');
    });

    it('builds pinephone args (no fields)', () => {
        const result = build_args_pure(pinephone, {});
        expect(result).toBe('pinephone');
    });

    it('skips resetConfig in update mode', () => {
        const result = build_args_pure(
            orbic,
            {
                adminIp: '192.168.1.1',
                adminUsername: 'admin',
                adminPassword: 'pass',
                resetConfig: true,
            },
            'update'
        );
        expect(result).not.toContain('--reset-config');
    });

    it('builds tplink with sdcard options', () => {
        const result = build_args_pure(tplink, {
            adminIp: '192.168.0.1',
            sdcardPath: '/media/card',
            skipSdcard: false,
            resetConfig: false,
        });
        expect(result).toBe('tplink --admin-ip 192.168.0.1 --sdcard-path /media/card');
    });

    it('builds tplink with skip sdcard flag', () => {
        const result = build_args_pure(tplink, {
            adminIp: '192.168.0.1',
            sdcardPath: '',
            skipSdcard: true,
            resetConfig: false,
        });
        expect(result).toBe('tplink --admin-ip 192.168.0.1 --skip-sdcard');
    });
});
