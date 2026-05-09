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

    function copy_output() {
        navigator.clipboard.writeText(output_log);
        copied = true;
        setTimeout(() => (copied = false), 2000);
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
            onclick={oncancel}
        >
            Cancel
        </button>
    </div>

    <ul class="space-y-1">
        {#each steps as step, i (i)}
            <li class="flex items-center gap-3 py-1">
                {#if step.status === 'done'}
                    <div
                        class="w-6 h-6 rounded-full bg-green-500 text-white flex items-center justify-center text-xs font-bold"
                    >
                        &check;
                    </div>
                {:else if step.status === 'active'}
                    <div
                        class="w-6 h-6 rounded-full bg-rayhunter-blue text-white flex items-center justify-center"
                    >
                        <div class="w-2 h-2 rounded-full bg-white animate-pulse"></div>
                    </div>
                {:else if step.status === 'error'}
                    <div
                        class="w-6 h-6 rounded-full bg-red-500 text-white flex items-center justify-center text-xs font-bold"
                    >
                        &times;
                    </div>
                {:else}
                    <div class="w-6 h-6 rounded-full border-2 border-gray-300"></div>
                {/if}
                <span
                    class:font-semibold={step.status === 'active'}
                    class:text-rayhunter-blue={step.status === 'active'}
                    class:text-gray-400={step.status === 'pending'}
                    class:text-red-500={step.status === 'error'}
                >
                    {step.label}{step.status === 'active' ? '...' : ''}
                </span>
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
