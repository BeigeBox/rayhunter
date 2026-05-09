<script lang="ts">
    let {
        admin_ip,
        verified,
        ondone,
        onanother,
    }: {
        admin_ip: string;
        verified: boolean;
        ondone: () => void;
        onanother: () => void;
    } = $props();
</script>

<div class="max-w-md mx-auto pt-12 px-4 space-y-4 text-center">
    {#if verified}
        <div
            class="w-16 h-16 mx-auto rounded-full bg-green-500 text-white flex items-center justify-center text-2xl font-bold"
        >
            &check;
        </div>
        <h2 class="text-2xl font-bold">Rayhunter is running!</h2>
    {:else}
        <div
            class="w-16 h-16 mx-auto rounded-full bg-yellow-400 text-white flex items-center justify-center text-2xl font-bold"
        >
            !
        </div>
        <h2 class="text-2xl font-bold">Installation complete</h2>
        <p class="text-gray-500">
            We couldn't confirm rayhunter is running. Try opening the link below in your browser. If
            it doesn't load, wait a minute for the device to finish booting.
        </p>
    {/if}
    {#if admin_ip}
        <a
            href="http://{admin_ip}:8080"
            target="_blank"
            class="inline-block px-4 py-2 rounded-lg bg-indigo-50 text-rayhunter-blue font-medium"
        >
            http://{admin_ip}:8080
        </a>
    {/if}
    <div class="flex gap-3 justify-center pt-4">
        <button
            class="cursor-pointer px-5 py-2 rounded-lg shadow-md bg-rayhunter-blue text-white"
            onclick={ondone}
        >
            Done
        </button>
        <button
            class="cursor-pointer px-5 py-2 rounded-lg shadow-md border border-rayhunter-blue text-rayhunter-blue bg-white"
            onclick={onanother}
        >
            Install another device
        </button>
    </div>
</div>
