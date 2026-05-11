import { invoke } from '@tauri-apps/api/core';
import type { DeviceId } from './types';

export type CheckStatus = 'pass' | 'warn' | 'fail' | 'pending';

export interface PreflightResult {
    reachable: { status: CheckStatus; label: string };
}

// Devices without an admin web UI; preflight is meaningless for these.
const USB_ONLY_DEVICES: DeviceId[] = ['orbic-usb', 'pinephone'];

export function device_skips_preflight(device_id: DeviceId): boolean {
    return USB_ONLY_DEVICES.includes(device_id);
}

export async function run_preflight(
    admin_ip: string,
    device_id?: DeviceId
): Promise<PreflightResult> {
    if (device_id && device_skips_preflight(device_id)) {
        return { reachable: { status: 'pass', label: 'USB device (no network check)' } };
    }
    const reachable = await check_device_reachable(admin_ip);
    return { reachable };
}

async function check_device_reachable(
    admin_ip: string
): Promise<{ status: CheckStatus; label: string }> {
    if (!admin_ip) {
        return { status: 'pass', label: 'No IP to check (USB device)' };
    }
    try {
        const ok = await invoke<boolean>('check_device_reachable', {
            ip: admin_ip,
            port: 80,
        });
        if (ok) {
            return { status: 'pass', label: `Reachable at ${admin_ip}` };
        }
        return {
            status: 'fail',
            label: `Could not reach device at ${admin_ip}. Are you connected to the hotspot WiFi?`,
        };
    } catch {
        return {
            status: 'fail',
            label: `Could not reach device at ${admin_ip}. Are you connected to the hotspot WiFi?`,
        };
    }
}
