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
    import DevToolbar from '../lib/components/DevToolbar.svelte';

    const installer = create_installer_state();

    const USB_DEVICES = ['orbic-usb', 'pinephone'];

    listen<string>('installer-output', (event) => {
        installer.append_output(event.payload);
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
        if (installer.screen.kind !== 'config') return;
        const device = installer.screen.device;
        installer.start_install(device, mode);

        try {
            await invoke('install_rayhunter', { args: installer.build_args(device, mode) });
            await finish_install(device);
        } catch (error) {
            installer.install_failed(device, classify_error(String(error)));
        }
    }

    async function finish_install(device: import('../lib/types').DeviceInfo) {
        if (USB_DEVICES.includes(device.id)) {
            installer.install_succeeded(device, true);
            return;
        }
        const admin_ip = installer.field_values['adminIp'] as string;
        if (!admin_ip) {
            installer.install_succeeded(device, false);
            return;
        }
        const verified = await verify_device(admin_ip);
        if (installer.screen.kind !== 'progress') return;
        installer.install_succeeded(device, verified);
    }

    async function cancel_install() {
        await invoke('cancel_installer');
    }

    function do_retry() {
        if (installer.screen.kind !== 'failure') return;
        const { device, args } = installer.screen;
        installer.retry(device, args);
        invoke('install_rayhunter', { args }).then(
            () => {
                if (installer.screen.kind === 'progress') finish_install(installer.screen.device);
            },
            (err) => {
                if (installer.screen.kind === 'progress')
                    installer.install_failed(installer.screen.device, classify_error(String(err)));
            }
        );
    }

    function copy_log() {
        if (installer.screen.kind !== 'failure') return;
        const header = `Device: ${installer.screen.device.label}\n---\n`;
        navigator.clipboard.writeText(header + installer.screen.log);
    }
</script>

{#if installer.screen.kind === 'device-select'}
    <DeviceSelect onselect={(device) => installer.select_device(device)} />
{:else if installer.screen.kind === 'config'}
    <ConfigForm
        device={installer.screen.device}
        bind:field_values={installer.field_values}
        oninstall={(mode) => run_install(mode)}
        onback={() => installer.reset()}
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
        ondone={() => installer.reset()}
        onanother={() => installer.reset()}
    />
{:else if installer.screen.kind === 'failure'}
    <FailureView error={installer.screen.error} onretry={do_retry} oncopylog={copy_log} />
{/if}

{#if installer.overlay === 'tplink-login'}
    <TpLinkOverlay />
{/if}

{#if import.meta.env.DEV}
    <DevToolbar {installer} />
{/if}
