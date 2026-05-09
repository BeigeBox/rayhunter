<script lang="ts">
    import type { DeviceInfo } from '../types';
    import { DEVICES } from '../devices';

    let { onselect }: { onselect: (device: DeviceInfo) => void } = $props();

    let selected_id = $state(DEVICES[0].id);
</script>

<div class="max-w-md mx-auto pt-8 px-4 space-y-6 text-center">
    <img src="/rayhunter_orca_only.png" alt="" class="h-24 mx-auto" />
    <h1 class="text-2xl font-bold">Install Rayhunter</h1>
    <p class="text-gray-500">Select your device to get started.</p>

    <select class="w-full px-3 py-2 rounded-lg shadow-md" bind:value={selected_id}>
        {#each DEVICES as device (device.id)}
            <option value={device.id}>{device.label}</option>
        {/each}
    </select>

    <button
        class="cursor-pointer px-6 py-2 rounded-lg shadow-md bg-rayhunter-blue text-white"
        onclick={() => {
            const device = DEVICES.find((d) => d.id === selected_id);
            if (device) onselect(device);
        }}
    >
        Next
    </button>
</div>
