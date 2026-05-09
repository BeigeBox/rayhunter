import type { DeviceInfo, Screen, InstallStep } from './types';
import { get_admin_ip } from './devices';
import { parse_output_line, mark_all_done, mark_active_error } from './output_parser';

export function create_installer_state() {
    let screen = $state<Screen>({ kind: 'device-select' });
    let output_log = $state('');
    let steps = $state<InstallStep[]>([]);
    let overlay = $state<'tplink-login' | null>(null);
    let field_values = $state<Record<string, string | boolean>>({});

    function select_device(device: DeviceInfo) {
        const defaults: Record<string, string | boolean> = {};
        for (const field of device.fields) {
            if (field.default_value !== undefined) {
                defaults[field.key] = field.default_value;
            } else {
                defaults[field.key] = field.type === 'checkbox' ? false : '';
            }
        }
        field_values = defaults;
        screen = { kind: 'config', device };
    }

    function build_args(device: DeviceInfo, mode: 'install' | 'update'): string {
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

    function start_install(device: DeviceInfo, mode: 'install' | 'update') {
        const args = build_args(device, mode);
        output_log = '';
        steps = device.steps.map((s, i) => ({
            label:
                mode === 'update' && s.label === 'Transferring files' ? 'Updating daemon' : s.label,
            status: i === 0 ? 'active' : 'pending',
        }));
        overlay = null;
        screen = { kind: 'progress', device, args, mode };
    }

    function append_output(text: string) {
        output_log += text;
        if (screen.kind !== 'progress') return;
        const lines = text.split('\n');
        for (const line of lines) {
            if (!line.trim()) continue;
            const prev_steps = steps;
            const result = parse_output_line(line, steps, screen.device.steps);
            steps = result.steps;
            if (result.overlay === 'tplink-login') {
                overlay = 'tplink-login';
            }
            if (overlay === 'tplink-login' && steps !== prev_steps) {
                overlay = null;
            }
        }
    }

    function set_overlay(value: 'tplink-login' | null) {
        overlay = value;
    }

    function install_succeeded(device: DeviceInfo, verified: boolean) {
        steps = mark_all_done(steps);
        const admin_ip = get_admin_ip(field_values);
        screen = { kind: 'success', device, admin_ip, verified };
    }

    function install_failed(
        device: DeviceInfo,
        error_guidance: { title: string; message: string; context?: string[] }
    ) {
        steps = mark_active_error(steps);
        const args = screen.kind === 'progress' ? screen.args : '';
        screen = {
            kind: 'failure',
            device,
            error: error_guidance,
            log: output_log,
            args,
        };
    }

    function retry(device: DeviceInfo, args: string) {
        output_log = '';
        steps = device.steps.map((s, i) => ({
            label: s.label,
            status: i === 0 ? 'active' : 'pending',
        }));
        overlay = null;
        screen = { kind: 'progress', device, args, mode: 'install' };
    }

    function reset() {
        screen = { kind: 'device-select' };
        output_log = '';
        steps = [];
        overlay = null;
        field_values = {};
    }

    return {
        get screen() {
            return screen;
        },
        get output_log() {
            return output_log;
        },
        get steps() {
            return steps;
        },
        get overlay() {
            return overlay;
        },
        get field_values() {
            return field_values;
        },
        set field_values(v: Record<string, string | boolean>) {
            field_values = v;
        },
        select_device,
        build_args,
        start_install,
        append_output,
        set_overlay,
        install_succeeded,
        install_failed,
        retry,
        reset,
    };
}

export type InstallerState = ReturnType<typeof create_installer_state>;
