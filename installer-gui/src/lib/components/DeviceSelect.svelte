<script lang="ts">
    import type { DeviceInfo } from '../types';
    import { DEVICES } from '../devices';

    let { onselect }: { onselect: (device: DeviceInfo) => void } = $props();

    let selected_id = $state(DEVICES[0].id);
    let select_el: HTMLSelectElement | undefined = $state();

    $effect(() => {
        select_el?.focus();
    });

    function handle_submit(event: Event) {
        event.preventDefault();
        const device = DEVICES.find((d) => d.id === selected_id);
        if (device) onselect(device);
    }
</script>

<form class="max-w-md mx-auto pt-8 px-4 space-y-6 text-center" onsubmit={handle_submit}>
    <img src="/rayhunter_orca_only.png" alt="" class="h-24 mx-auto" />
    <h1 class="text-2xl font-bold">Install Rayhunter</h1>
    <p class="text-gray-500">Select your device to get started.</p>

    <select
        class="w-full px-3 py-2 rounded-lg shadow-md"
        bind:value={selected_id}
        bind:this={select_el}
    >
        {#each DEVICES as device (device.id)}
            <option value={device.id}>{device.label}</option>
        {/each}
    </select>

    <button
        type="submit"
        class="cursor-pointer px-6 py-2 rounded-lg shadow-md bg-rayhunter-blue text-white"
    >
        Next
    </button>
</form>
