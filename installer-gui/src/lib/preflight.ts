import { invoke } from '@tauri-apps/api/core';

export type CheckStatus = 'pass' | 'warn' | 'fail' | 'pending';

export interface PreflightResult {
    reachable: { status: CheckStatus; label: string };
}

export async function run_preflight(admin_ip: string): Promise<PreflightResult> {
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
            return { status: 'pass', label: `Device reachable at ${admin_ip}` };
        }
        return {
            status: 'fail',
            label: `Can't reach device at ${admin_ip}. Are you connected to the hotspot WiFi?`,
        };
    } catch {
        return {
            status: 'fail',
            label: `Can't reach device at ${admin_ip}. Are you connected to the hotspot WiFi?`,
        };
    }
}
