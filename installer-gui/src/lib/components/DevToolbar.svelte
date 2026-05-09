<script lang="ts">
    import type { InstallerState } from '../state.svelte';
    import { DEVICES } from '../devices';
    import { jump_to, mock_run, FAILURE_SCENARIOS, type MockOutcome } from '../mock_install';

    let { installer }: { installer: InstallerState } = $props();

    let open = $state(false);
    let device_index = $state(0);
    let cancel_mock = $state<(() => void) | null>(null);

    const SCREENS = [
        'device-select',
        'config',
        'progress',
        'success-verified',
        'success-unverified',
        'tplink-overlay',
        ...FAILURE_SCENARIOS.map((_, i) => `failure-${i}`),
    ];

    const SCREEN_LABELS: Record<string, string> = {
        'device-select': 'Device Select',
        config: 'Config Form',
        progress: 'Progress (mid-install)',
        'success-verified': 'Success (verified)',
        'success-unverified': 'Success (unverified)',
        'tplink-overlay': 'TP-Link Overlay',
    };
    for (let i = 0; i < FAILURE_SCENARIOS.length; i++) {
        SCREEN_LABELS[`failure-${i}`] = `Fail: ${FAILURE_SCENARIOS[i].label}`;
    }

    function go(screen: string) {
        stop_mock();
        const device = DEVICES[device_index];
        const failIdx = screen.startsWith('failure-') ? parseInt(screen.split('-')[1]) : undefined;
        jump_to(installer, screen.replace(/-\d+$/, ''), device, failIdx);
    }

    function run(outcome: MockOutcome) {
        stop_mock();
        cancel_mock = mock_run(installer, DEVICES[device_index], outcome);
    }

    function stop_mock() {
        if (cancel_mock) {
            cancel_mock();
            cancel_mock = null;
        }
    }
</script>

<div class="fixed bottom-12 right-2 z-50">
    {#if open}
        <div
            class="bg-white border border-gray-300 rounded-lg shadow-lg p-3 w-64 space-y-2 text-sm"
        >
            <div class="flex justify-between items-center">
                <span class="font-bold text-xs text-gray-500 uppercase tracking-wide"
                    >Dev Tools</span
                >
                <button class="cursor-pointer text-gray-400 text-xs" onclick={() => (open = false)}>
                    close
                </button>
            </div>

            <div>
                <label class="block text-xs text-gray-500 mb-0.5" for="dev-device">Device</label>
                <select
                    id="dev-device"
                    class="w-full px-2 py-1 border rounded text-xs"
                    bind:value={device_index}
                >
                    {#each DEVICES as d, i (d.id)}
                        <option value={i}>{d.label}</option>
                    {/each}
                </select>
            </div>

            <div>
                <label class="block text-xs text-gray-500 mb-0.5" for="dev-screen"
                    >Jump to screen</label
                >
                <select
                    id="dev-screen"
                    class="w-full px-2 py-1 border rounded text-xs"
                    onchange={(e) => go(e.currentTarget.value)}
                >
                    <option value="" disabled selected>Pick a screen...</option>
                    {#each SCREENS as s (s)}
                        <option value={s}>{SCREEN_LABELS[s]}</option>
                    {/each}
                </select>
            </div>

            <div class="border-t border-gray-200 pt-2 space-y-1">
                <p class="text-xs text-gray-500">Run mock install</p>
                <div class="flex flex-wrap gap-1">
                    <button
                        class="cursor-pointer px-2 py-0.5 rounded border text-xs bg-green-50 border-green-300 text-green-700"
                        onclick={() => run({ kind: 'success-verified' })}
                    >
                        Success
                    </button>
                    <button
                        class="cursor-pointer px-2 py-0.5 rounded border text-xs bg-yellow-50 border-yellow-300 text-yellow-700"
                        onclick={() => run({ kind: 'success-unverified' })}
                    >
                        Unverified
                    </button>
                    <button
                        class="cursor-pointer px-2 py-0.5 rounded border text-xs bg-red-50 border-red-300 text-red-700"
                        onclick={() =>
                            run({
                                kind: 'failure',
                                error: FAILURE_SCENARIOS[0].error,
                            })}
                    >
                        Fail
                    </button>
                    {#if cancel_mock}
                        <button
                            class="cursor-pointer px-2 py-0.5 rounded border text-xs bg-gray-100 border-gray-300"
                            onclick={stop_mock}
                        >
                            Stop
                        </button>
                    {/if}
                </div>
            </div>

            <div class="text-[10px] text-gray-400">
                Screen: {installer.screen.kind}
            </div>
        </div>
    {:else}
        <button
            class="cursor-pointer px-2 py-1 rounded bg-gray-800 text-white text-xs opacity-50 hover:opacity-100"
            onclick={() => (open = true)}
        >
            DEV
        </button>
    {/if}
</div>
