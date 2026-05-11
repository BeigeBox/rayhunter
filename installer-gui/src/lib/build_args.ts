import type { DeviceInfo, InstallMode } from './types';
import { FIELD_APPLIES_TO } from './devices';

export interface InstallCommand {
    args: string[];
    password: string;
}

function field_applies(device_id: string, field_key: string, mode: InstallMode): boolean {
    const modes = FIELD_APPLIES_TO[`${device_id}.${field_key}`];
    return modes ? modes.includes(mode) : true;
}

export function build_install_command(
    device: DeviceInfo,
    field_values: Record<string, string | boolean>,
    mode: InstallMode
): InstallCommand {
    const args: string[] = [device.command];
    let password = '';

    for (const field of device.fields) {
        if (!field_applies(device.id, field.key, mode)) continue;
        const val = field_values[field.key];

        if (field.type === 'checkbox') {
            if (val) args.push(field.arg_name);
            continue;
        }

        if (typeof val !== 'string') continue;
        const trimmed = val.trim();
        if (!trimmed) continue;

        if (field.arg_name === '--admin-password') {
            password = trimmed;
            continue;
        }

        args.push(field.arg_name, trimmed);
    }

    return { args, password };
}
