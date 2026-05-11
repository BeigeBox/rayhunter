import type {
    DeviceInfo,
    InstallMode,
    InstallStep,
    Overlay,
    Screen,
    StepDefinition,
} from './types';
import { build_install_command, type InstallCommand } from './build_args';
import {
    create_output_parser,
    mark_all_done,
    mark_active_error,
    type OutputParser,
} from './output_parser';

export class InstallerState {
    screen = $state<Screen>({ kind: 'device-select' });
    output_log = $state('');
    steps = $state<InstallStep[]>([]);
    overlay = $state<Overlay | null>(null);
    field_values = $state<Record<string, string | boolean>>({});

    private parser: OutputParser = create_output_parser();

    select_device(device: DeviceInfo): void {
        const defaults: Record<string, string | boolean> = {};
        for (const field of device.fields) {
            if (field.default_value !== undefined) {
                defaults[field.key] = field.default_value;
            } else {
                defaults[field.key] = field.type === 'checkbox' ? false : '';
            }
        }
        this.field_values = defaults;
        this.screen = { kind: 'config', device };
    }

    build_install_command(): InstallCommand {
        if (this.screen.kind !== 'config' && this.screen.kind !== 'progress') {
            throw new Error(`build_install_command called from screen kind '${this.screen.kind}'`);
        }
        const mode = this.screen.kind === 'progress' ? this.screen.mode : 'install';
        return build_install_command(this.screen.device, this.field_values, mode);
    }

    start_install(device: DeviceInfo, mode: InstallMode): void {
        const { args } = build_install_command(device, this.field_values, mode);
        this.output_log = '';
        this.steps = make_initial_steps(device.steps, mode);
        this.overlay = null;
        this.parser = create_output_parser();
        this.screen = { kind: 'progress', device, args, mode };
    }

    append_output(text: string): void {
        this.output_log += text;
        if (this.screen.kind !== 'progress') return;
        const prev_steps = this.steps;
        const result = this.parser.feed(text, this.steps, this.screen.device.steps);
        this.steps = result.steps;
        if (result.overlay_action) {
            this.overlay = result.overlay_action;
        }
        if (this.overlay?.type === 'tplink_browser' && this.steps !== prev_steps) {
            this.overlay = null;
        }
    }

    set_overlay(overlay: Overlay): void {
        this.overlay = overlay;
    }

    clear_overlay(): void {
        this.overlay = null;
    }

    install_succeeded(verified: boolean): void {
        this.steps = mark_all_done(this.steps);
        const admin_ip = (this.field_values['adminIp'] as string | undefined) ?? '';
        this.screen = { kind: 'success', admin_ip, verified };
    }

    install_failed(error: string, args: string[]): void {
        if (this.screen.kind !== 'progress') {
            throw new Error(`install_failed called from screen kind '${this.screen.kind}'`);
        }
        const { device, mode } = this.screen;
        this.steps = mark_active_error(this.steps);
        this.screen = {
            kind: 'failure',
            device,
            error,
            log: this.output_log,
            args,
            mode,
        };
    }

    retry(): void {
        if (this.screen.kind !== 'failure') {
            throw new Error(`retry called from screen kind '${this.screen.kind}'`);
        }
        const { device, args, mode } = this.screen;
        this.output_log = '';
        this.steps = make_initial_steps(device.steps, mode);
        this.overlay = null;
        this.parser = create_output_parser();
        this.screen = { kind: 'progress', device, args, mode };
    }

    reset(): void {
        this.screen = { kind: 'device-select' };
        this.output_log = '';
        this.steps = [];
        this.overlay = null;
        this.field_values = {};
        this.parser = create_output_parser();
    }
}

function make_initial_steps(defs: StepDefinition[], mode: InstallMode): InstallStep[] {
    return defs.map((s, i) => ({
        label: mode === 'update' && s.label === 'Transferring files' ? 'Updating daemon' : s.label,
        status: i === 0 ? 'active' : 'pending',
    }));
}

export function create_installer_state(): InstallerState {
    return new InstallerState();
}
