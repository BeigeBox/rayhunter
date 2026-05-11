import { describe, it, expect } from 'vitest';
import { DEVICES } from './devices';
import { build_install_command } from './build_args';
import { InstallerState } from './state.svelte';

const orbic = DEVICES.find((d) => d.id === 'orbic')!;
const tplink = DEVICES.find((d) => d.id === 'tplink')!;
const pinephone = DEVICES.find((d) => d.id === 'pinephone')!;
const uz801 = DEVICES.find((d) => d.id === 'uz801')!;

describe('devices', () => {
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

    it('every null-marker step is the final Verifying step', () => {
        for (const device of DEVICES) {
            for (const step of device.steps) {
                if (step.marker === null) {
                    expect(step.label).toBe('Verifying');
                }
            }
        }
    });
});

describe('build_install_command', () => {
    it('separates password from args for orbic', () => {
        const { args, password } = build_install_command(
            orbic,
            {
                adminIp: '192.168.1.1',
                adminUsername: 'admin',
                adminPassword: 'secret',
                resetConfig: true,
            },
            'install'
        );
        expect(password).toBe('secret');
        expect(args).toEqual([
            'orbic',
            '--admin-username',
            'admin',
            '--admin-ip',
            '192.168.1.1',
            '--reset-config',
        ]);
        expect(args).not.toContain('--admin-password');
        expect(args).not.toContain('secret');
    });

    it('returns empty password when none entered', () => {
        const { args, password } = build_install_command(
            orbic,
            {
                adminIp: '192.168.1.1',
                adminUsername: '',
                adminPassword: '',
                resetConfig: false,
            },
            'install'
        );
        expect(password).toBe('');
        expect(args).toEqual(['orbic', '--admin-ip', '192.168.1.1']);
    });

    it('skips empty string fields', () => {
        const { args } = build_install_command(
            orbic,
            {
                adminIp: '192.168.1.1',
                adminUsername: '',
                adminPassword: 'pass',
                resetConfig: false,
            },
            'install'
        );
        expect(args).toEqual(['orbic', '--admin-ip', '192.168.1.1']);
    });

    it('builds pinephone args with no fields', () => {
        const { args, password } = build_install_command(pinephone, {}, 'install');
        expect(args).toEqual(['pinephone']);
        expect(password).toBe('');
    });

    it('omits --reset-config in update mode', () => {
        const { args } = build_install_command(
            orbic,
            {
                adminIp: '192.168.1.1',
                adminUsername: 'admin',
                adminPassword: 'pass',
                resetConfig: true,
            },
            'update'
        );
        expect(args).not.toContain('--reset-config');
    });

    it('keeps non-reset checkboxes in update mode', () => {
        const { args } = build_install_command(
            tplink,
            {
                adminIp: '192.168.0.1',
                sdcardPath: '',
                skipSdcard: true,
                resetConfig: true,
            },
            'update'
        );
        expect(args).toContain('--skip-sdcard');
        expect(args).not.toContain('--reset-config');
    });

    it('builds tplink with sdcard options', () => {
        const { args } = build_install_command(
            tplink,
            {
                adminIp: '192.168.0.1',
                sdcardPath: '/media/card',
                skipSdcard: false,
                resetConfig: false,
            },
            'install'
        );
        expect(args).toEqual([
            'tplink',
            '--admin-ip',
            '192.168.0.1',
            '--sdcard-path',
            '/media/card',
        ]);
    });

    it('trims whitespace from values', () => {
        const { args, password } = build_install_command(
            orbic,
            {
                adminIp: '  192.168.1.1  ',
                adminUsername: '  admin  ',
                adminPassword: '  pw  ',
                resetConfig: false,
            },
            'install'
        );
        expect(password).toBe('pw');
        expect(args).toContain('192.168.1.1');
        expect(args).toContain('admin');
    });
});

describe('InstallerState', () => {
    it('starts on device-select with empty field_values', () => {
        const s = new InstallerState();
        expect(s.screen.kind).toBe('device-select');
        expect(s.field_values).toEqual({});
        expect(s.steps).toEqual([]);
        expect(s.output_log).toBe('');
        expect(s.overlay).toBeNull();
    });

    it('select_device transitions to config and seeds defaults', () => {
        const s = new InstallerState();
        s.select_device(orbic);
        expect(s.screen.kind).toBe('config');
        if (s.screen.kind === 'config') {
            expect(s.screen.device.id).toBe('orbic');
        }
        expect(s.field_values['adminUsername']).toBe('admin');
        expect(s.field_values['adminIp']).toBe('192.168.1.1');
        expect(s.field_values['resetConfig']).toBe(false);
        expect(s.field_values['adminPassword']).toBe('');
    });

    it('start_install enters progress with empty log and active first step', () => {
        const s = new InstallerState();
        s.select_device(orbic);
        s.field_values['adminPassword'] = 'pw';
        s.start_install(orbic, 'install');
        expect(s.screen.kind).toBe('progress');
        expect(s.output_log).toBe('');
        expect(s.steps).toHaveLength(orbic.steps.length);
        expect(s.steps[0].status).toBe('active');
        expect(s.steps.slice(1).every((step) => step.status === 'pending')).toBe(true);
        if (s.screen.kind === 'progress') {
            expect(s.screen.mode).toBe('install');
            expect(s.screen.args).not.toContain('--admin-password');
            expect(s.screen.args).not.toContain('pw');
        }
    });

    it('append_output accumulates lines and advances steps on marker hits', () => {
        const s = new InstallerState();
        s.select_device(orbic);
        s.start_install(orbic, 'install');
        s.append_output('hello\n');
        s.append_output('world\n');
        expect(s.output_log).toBe('hello\nworld\n');
        expect(s.steps[0].status).toBe('active');

        s.append_output('Waiting for telnet to become available\n');
        expect(s.steps[0].status).toBe('done');
        expect(s.steps[1].status).toBe('active');
    });

    it('install_failed records args and original mode', () => {
        const s = new InstallerState();
        s.select_device(orbic);
        s.start_install(orbic, 'update');
        s.install_failed('boom', ['orbic', '--admin-ip', '1.2.3.4']);
        expect(s.screen.kind).toBe('failure');
        if (s.screen.kind === 'failure') {
            expect(s.screen.error).toBe('boom');
            expect(s.screen.args).toEqual(['orbic', '--admin-ip', '1.2.3.4']);
            expect(s.screen.mode).toBe('update');
            expect(s.screen.log).toBe('');
        }
        expect(s.steps[0].status).toBe('error');
    });

    it('install_failed throws when not on progress screen', () => {
        const s = new InstallerState();
        s.select_device(orbic);
        expect(() => s.install_failed('boom', [])).toThrow();
    });

    it('retry preserves original mode and args', () => {
        const s = new InstallerState();
        s.select_device(orbic);
        s.start_install(orbic, 'update');
        const original_args = s.screen.kind === 'progress' ? [...s.screen.args] : [];
        s.install_failed('err', original_args);
        s.retry();
        expect(s.screen.kind).toBe('progress');
        if (s.screen.kind === 'progress') {
            expect(s.screen.mode).toBe('update');
            expect(s.screen.args).toEqual(original_args);
        }
        expect(s.output_log).toBe('');
        expect(s.steps[0].status).toBe('active');
    });

    it('retry throws when not on failure screen', () => {
        const s = new InstallerState();
        expect(() => s.retry()).toThrow();
    });

    it('install_succeeded marks all steps done and stores admin_ip', () => {
        const s = new InstallerState();
        s.select_device(orbic);
        s.start_install(orbic, 'install');
        s.install_succeeded(true);
        expect(s.screen.kind).toBe('success');
        if (s.screen.kind === 'success') {
            expect(s.screen.verified).toBe(true);
            expect(s.screen.admin_ip).toBe('192.168.1.1');
        }
        expect(s.steps.every((st) => st.status === 'done')).toBe(true);
    });

    it('install_succeeded with no adminIp returns empty admin_ip', () => {
        const s = new InstallerState();
        s.select_device(pinephone);
        s.start_install(pinephone, 'install');
        s.install_succeeded(false);
        if (s.screen.kind === 'success') {
            expect(s.screen.admin_ip).toBe('');
            expect(s.screen.verified).toBe(false);
        }
    });

    it('reset clears everything and returns to device-select', () => {
        const s = new InstallerState();
        s.select_device(orbic);
        s.field_values['adminPassword'] = 'secret';
        s.start_install(orbic, 'install');
        s.append_output('some output\n');
        s.reset();
        expect(s.screen.kind).toBe('device-select');
        expect(s.field_values).toEqual({});
        expect(s.steps).toEqual([]);
        expect(s.output_log).toBe('');
        expect(s.overlay).toBeNull();
    });

    it('set_overlay and clear_overlay manage overlay state', () => {
        const s = new InstallerState();
        s.set_overlay({ type: 'tplink_browser', url: 'http://127.0.0.1:4000' });
        expect(s.overlay).toEqual({ type: 'tplink_browser', url: 'http://127.0.0.1:4000' });
        s.clear_overlay();
        expect(s.overlay).toBeNull();
    });

    it('append_output extracts URL from listening line and sets overlay', () => {
        const s = new InstallerState();
        s.select_device(tplink);
        s.start_install(tplink, 'install');
        s.append_output('Listening on http://127.0.0.1:51234\n');
        expect(s.overlay).toEqual({
            type: 'tplink_browser',
            url: 'http://127.0.0.1:51234',
        });
    });

    it('append_output buffers partial lines across emits', () => {
        const s = new InstallerState();
        s.select_device(tplink);
        s.start_install(tplink, 'install');
        s.append_output('Listening on http');
        expect(s.overlay).toBeNull();
        s.append_output('://127.0.0.1:4321\n');
        expect(s.overlay).toEqual({
            type: 'tplink_browser',
            url: 'http://127.0.0.1:4321',
        });
    });

    it('build_install_command throws outside config or progress screen', () => {
        const s = new InstallerState();
        expect(() => s.build_install_command()).toThrow();
    });

    it('start_install does not retain password in screen.args', () => {
        const s = new InstallerState();
        s.select_device(orbic);
        s.field_values['adminPassword'] = 'super-secret';
        s.start_install(orbic, 'install');
        if (s.screen.kind === 'progress') {
            expect(s.screen.args.join(' ')).not.toContain('super-secret');
        }
    });
});
