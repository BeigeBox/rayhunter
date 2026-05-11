<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '@tauri-apps/api/event';
    import { create_installer_state } from '../lib/state.svelte';
    import { classify_error } from '../lib/error_classifier';
    import DeviceSelect from '../lib/components/DeviceSelect.svelte';
    import ConfigForm from '../lib/components/ConfigForm.svelte';
    import ProgressView from '../lib/components/ProgressView.svelte';
    import SuccessView from '../lib/components/SuccessView.svelte';
    import FailureView from '../lib/components/FailureView.svelte';
    import TpLinkOverlay from '../lib/components/TpLinkOverlay.svelte';
    import type { Component } from 'svelte';
    import type { InstallerState } from '../lib/state.svelte';

    const installer = create_installer_state();

    let DevToolbar = $state<Component<{ installer: InstallerState }> | null>(null);
    $effect(() => {
        if (import.meta.env.DEV) {
            import('../lib/components/DevToolbar.svelte').then((m) => {
                DevToolbar = m.default;
            });
        }
    });

    const USB_DEVICES = ['orbic-usb', 'pinephone'];
    let retry_count = $state(0);

    $effect(() => {
        const unlisten_promise = listen<string>('installer-output', (event) => {
            installer.append_output(event.payload);
        });
        return () => {
            unlisten_promise.then((unlisten) => unlisten());
        };
    });

    async function verify_device(admin_ip: string): Promise<boolean> {
        for (let i = 0; i < 10; i++) {
            try {
                const ok = await invoke<boolean>('check_device_reachable', {
                    ip: admin_ip,
                    port: 8080,
                });
                if (ok) return true;
            } catch {
                // ignore
            }
            await new Promise((r) => setTimeout(r, 3000));
        }
        return false;
    }

    async function run_install(mode: 'install' | 'update') {
        const start_screen = installer.screen;
        if (start_screen.kind !== 'config') return;
        const device = start_screen.device;
        retry_count = 0;
        installer.start_install(device, mode);

        const { args, password } = installer.build_install_command();
        try {
            await invoke('install_rayhunter', { args, password });
            await finish_install(device);
        } catch (error) {
            if (installer.screen.kind === 'progress') {
                installer.install_failed(String(error), args);
            }
        }
    }

    async function finish_install(device: import('../lib/types').DeviceInfo) {
        if (USB_DEVICES.includes(device.id)) {
            installer.install_succeeded(true);
            return;
        }
        const admin_ip = installer.field_values['adminIp'] as string;
        if (!admin_ip) {
            installer.install_succeeded(false);
            return;
        }
        const verified = await verify_device(admin_ip);
        if (installer.screen.kind !== 'progress') return;
        installer.install_succeeded(verified);
    }

    async function cancel_install() {
        await invoke('cancel_installer');
    }

    function do_retry() {
        const failure_screen = installer.screen;
        if (failure_screen.kind !== 'failure') return;
        const args = failure_screen.args;
        const device = failure_screen.device;
        retry_count += 1;
        installer.retry();
        if (installer.screen.kind !== 'progress') return;
        const { password } = installer.build_install_command();
        invoke('install_rayhunter', { args, password }).then(
            () => {
                if (installer.screen.kind === 'progress') finish_install(device);
            },
            (err) => {
                if (installer.screen.kind === 'progress') {
                    installer.install_failed(String(err), args);
                }
            }
        );
    }

    function redact(text: string): string {
        return text
            .replace(/--admin-password\s+\S+/gi, '--admin-password ***')
            .replace(/--password\s+\S+/gi, '--password ***')
            .replace(/--token\s+\S+/gi, '--token ***')
            .replace(/^(\s*Password:\s*)\S+/gim, '$1***');
    }

    function copy_log() {
        if (installer.screen.kind !== 'failure') return;
        const header = `Device: ${installer.screen.device.label}\n---\n`;
        navigator.clipboard.writeText(header + redact(installer.screen.log));
    }

    async function recheck_device(): Promise<boolean> {
        if (installer.screen.kind !== 'success') return false;
        const ip = installer.screen.admin_ip;
        if (!ip) return false;
        try {
            const ok = await invoke<boolean>('check_device_reachable', {
                ip,
                port: 8080,
            });
            if (ok && installer.screen.kind === 'success') {
                installer.install_succeeded(true);
            }
            return ok;
        } catch {
            return false;
        }
    }

    function reset_all() {
        retry_count = 0;
        installer.reset();
    }
</script>

{#if installer.screen.kind === 'device-select'}
    <DeviceSelect onselect={(device) => installer.select_device(device)} />
{:else if installer.screen.kind === 'config'}
    <ConfigForm
        device={installer.screen.device}
        bind:field_values={installer.field_values}
        oninstall={(mode) => run_install(mode)}
        onback={reset_all}
    />
{:else if installer.screen.kind === 'progress'}
    <ProgressView
        device={installer.screen.device}
        mode={installer.screen.mode}
        steps={installer.steps}
        output_log={installer.output_log}
        oncancel={cancel_install}
    />
{:else if installer.screen.kind === 'success'}
    <SuccessView
        admin_ip={installer.screen.admin_ip}
        verified={installer.screen.verified}
        ondone={reset_all}
        onanother={reset_all}
        oncheckagain={recheck_device}
    />
{:else if installer.screen.kind === 'failure'}
    <FailureView
        device={installer.screen.device}
        mode={installer.screen.mode}
        error={classify_error(installer.screen.error)}
        log={installer.screen.log}
        {retry_count}
        onretry={do_retry}
        oncopylog={copy_log}
    />
{/if}

{#if installer.overlay?.type === 'tplink_browser'}
    <TpLinkOverlay url={installer.overlay.url} oncancel={cancel_install} />
{/if}

{#if DevToolbar}
    <DevToolbar {installer} />
{/if}
