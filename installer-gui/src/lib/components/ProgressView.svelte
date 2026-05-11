<script lang="ts">
    import type { DeviceInfo, InstallStep } from '../types';

    let {
        device,
        mode,
        steps,
        output_log,
        oncancel,
    }: {
        device: DeviceInfo;
        mode: 'install' | 'update';
        steps: InstallStep[];
        output_log: string;
        oncancel: () => void;
    } = $props();

    let copied = $state(false);
    let confirming_cancel = $state(false);
    let now = $state(Date.now());
    let last_output_at = $state(Date.now());
    let step_started_at = $state(Date.now());
    let last_active_index = -1;

    const STALL_THRESHOLD_MS = 30_000;

    const active_index = $derived(steps.findIndex((s) => s.status === 'active'));
    const elapsed_ms = $derived(active_index >= 0 ? now - step_started_at : 0);
    const stalled = $derived(active_index >= 0 && now - last_output_at > STALL_THRESHOLD_MS);

    $effect(() => {
        if (active_index !== last_active_index) {
            last_active_index = active_index;
            step_started_at = Date.now();
        }
    });

    $effect(() => {
        // Track output_log to re-run on every append.
        if (output_log.length > 0) {
            last_output_at = Date.now();
        }
    });

    $effect(() => {
        const id = setInterval(() => {
            now = Date.now();
        }, 1000);
        return () => clearInterval(id);
    });

    function format_elapsed(ms: number): string {
        const total_s = Math.floor(ms / 1000);
        if (total_s < 60) return `${total_s}s`;
        const m = Math.floor(total_s / 60);
        const s = total_s % 60;
        return s === 0 ? `${m}m` : `${m}m ${s}s`;
    }

    function copy_output() {
        const redacted = output_log
            .replace(/--admin-password\s+\S+/gi, '--admin-password ***')
            .replace(/--password\s+\S+/gi, '--password ***');
        navigator.clipboard.writeText(redacted);
        copied = true;
        setTimeout(() => (copied = false), 2000);
    }

    function request_cancel() {
        confirming_cancel = true;
    }

    function confirm_cancel() {
        confirming_cancel = false;
        oncancel();
    }

    function keep_installing() {
        confirming_cancel = false;
    }

    function step_status_label(status: string): string {
        if (status === 'done') return 'Done';
        if (status === 'active') return 'In progress';
        if (status === 'error') return 'Failed';
        return 'Pending';
    }

    function step_tooltip(label: string): string | undefined {
        if (label === 'Connecting') return 'Authenticating with the device admin panel.';
        if (label === 'Rooting') return 'Unlocking the device shell to install Rayhunter.';
        if (label === 'Transferring files') return 'Copying the Rayhunter binaries to the device.';
        if (label === 'Rebooting') return 'Restarting the device so Rayhunter starts on boot.';
        if (label === 'Verifying') return 'Confirming the Rayhunter web interface is responding.';
        if (label === 'Updating daemon') return 'Copying the updated daemon to the device.';
        return undefined;
    }
</script>

<div class="max-w-md mx-auto pt-5 px-4 space-y-4">
    <div class="flex justify-between items-center">
        <h2 class="text-xl font-semibold">
            {mode === 'update' ? 'Updating' : 'Installing on'}
            {device.label}
        </h2>
        <button
            class="cursor-pointer px-4 py-1 rounded-lg border border-red-500 text-red-500 text-sm"
            onclick={request_cancel}
        >
            Cancel
        </button>
    </div>

    <ul class="space-y-1" aria-live="polite">
        {#each steps as step, i (i)}
            {@const tooltip = step_tooltip(step.label)}
            <li class="flex items-start gap-3 py-1">
                {#if step.status === 'done'}
                    <div
                        class="w-6 h-6 rounded-full bg-green-500 text-white flex items-center justify-center text-xs font-bold shrink-0"
                        aria-label={step_status_label(step.status)}
                    >
                        &check;
                    </div>
                {:else if step.status === 'active'}
                    <div
                        class="w-6 h-6 rounded-full bg-rayhunter-blue text-white flex items-center justify-center shrink-0"
                        aria-label={step_status_label(step.status)}
                    >
                        <div class="w-2 h-2 rounded-full bg-white animate-pulse"></div>
                    </div>
                {:else if step.status === 'error'}
                    <div
                        class="w-6 h-6 rounded-full bg-red-500 text-white flex items-center justify-center text-xs font-bold shrink-0"
                        aria-label={step_status_label(step.status)}
                    >
                        &times;
                    </div>
                {:else}
                    <div
                        class="w-6 h-6 rounded-full border-2 border-gray-300 shrink-0"
                        aria-label={step_status_label(step.status)}
                    ></div>
                {/if}
                <div class="flex-1 min-w-0">
                    <div class="flex items-baseline gap-2 flex-wrap">
                        <span
                            class:font-semibold={step.status === 'active'}
                            class:text-rayhunter-blue={step.status === 'active'}
                            class:text-gray-400={step.status === 'pending'}
                            class:text-red-500={step.status === 'error'}
                            title={tooltip}
                        >
                            {step.label}{step.status === 'active' ? '...' : ''}
                        </span>
                        {#if step.status === 'active' && elapsed_ms >= 1000}
                            <span class="text-xs text-gray-400">{format_elapsed(elapsed_ms)}</span>
                        {/if}
                    </div>
                    {#if step.status === 'active' && stalled}
                        <p class="text-xs text-yellow-700 mt-0.5">
                            This step is taking longer than usual. Keep waiting, or click Cancel to
                            stop.
                        </p>
                    {/if}
                </div>
            </li>
        {/each}
    </ul>

    <div class="bg-gray-900 rounded-lg p-3 min-h-24 overflow-auto resize terminal-box">
        <div class="sticky top-0 float-right z-10 -mt-1.5 -mr-1 mb-1">
            <button
                class="cursor-pointer px-2 py-0.5 rounded text-[10px] bg-gray-700 text-gray-300 hover:bg-gray-600"
                onclick={copy_output}
            >
                {copied ? 'Copied!' : 'Copy'}
            </button>
        </div>
        <pre class="text-xs text-gray-300 whitespace-pre-wrap">{output_log.slice(-500)}</pre>
    </div>

    <div class="relative overflow-hidden h-12">
        <img src="/rayhunter_orca_only.png" alt="" class="h-12 absolute orca-swim" />
    </div>
</div>

{#if confirming_cancel}
    <div
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
        role="dialog"
        aria-modal="true"
        aria-labelledby="cancel-confirm-title"
    >
        <div class="bg-white rounded-xl shadow-xl max-w-sm mx-4 p-6 space-y-4">
            <h3 id="cancel-confirm-title" class="text-lg font-semibold">Cancel installation?</h3>
            <p class="text-sm text-gray-600">
                Cancelling now may leave your device in a partial state. Some steps can't be safely
                interrupted (mid-USB-write, mid-HTTP), so cancellation may take a moment to take
                effect.
            </p>
            <div class="flex justify-end gap-2">
                <button
                    type="button"
                    class="cursor-pointer px-4 py-2 rounded-lg border border-gray-300 text-gray-700 bg-white"
                    onclick={keep_installing}
                >
                    Keep installing
                </button>
                <button
                    type="button"
                    class="cursor-pointer px-4 py-2 rounded-lg shadow-md bg-red-500 text-white"
                    onclick={confirm_cancel}
                >
                    Cancel install
                </button>
            </div>
        </div>
    </div>
{/if}
