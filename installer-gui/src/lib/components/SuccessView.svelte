<script lang="ts">
    let {
        admin_ip,
        verified,
        ondone,
        onanother,
        oncheckagain,
    }: {
        admin_ip: string;
        verified: boolean;
        ondone: () => void;
        onanother: () => void;
        oncheckagain: () => Promise<boolean>;
    } = $props();

    let checking = $state(false);
    let check_result = $state<'ok' | 'still_unreachable' | null>(null);

    const is_usb_only = $derived(!admin_ip);

    async function check_again() {
        checking = true;
        check_result = null;
        const ok = await oncheckagain();
        checking = false;
        check_result = ok ? 'ok' : 'still_unreachable';
    }
</script>

<div class="max-w-md mx-auto pt-12 px-4 space-y-4 text-center">
    {#if verified}
        <div
            class="w-16 h-16 mx-auto rounded-full bg-green-500 text-white flex items-center justify-center text-2xl font-bold"
            aria-hidden="true"
        >
            &check;
        </div>
        <h2 class="text-2xl font-bold">Rayhunter is running</h2>
    {:else}
        <div
            class="w-16 h-16 mx-auto rounded-full bg-yellow-400 text-white flex items-center justify-center text-2xl font-bold"
            aria-hidden="true"
        >
            !
        </div>
        <h2 class="text-2xl font-bold">Installation complete</h2>
        {#if is_usb_only}
            <p class="text-gray-500">
                The device was installed over USB. Reconnect to your phone's WiFi or check the
                Rayhunter web UI over USB once the device finishes rebooting.
            </p>
        {:else}
            <p class="text-gray-500">
                We couldn't confirm Rayhunter is running. Try opening the link below in your
                browser. If it doesn't load, wait a minute for the device to finish booting, then
                check again.
            </p>
        {/if}
    {/if}

    {#if admin_ip}
        <a
            href="http://{admin_ip}:8080"
            target="_blank"
            rel="noreferrer"
            class="inline-block px-4 py-2 rounded-lg bg-indigo-50 text-rayhunter-blue font-medium"
        >
            http://{admin_ip}:8080
        </a>
    {/if}

    {#if !verified && !is_usb_only}
        <div>
            <button
                class="cursor-pointer px-4 py-2 rounded-lg shadow-md border border-rayhunter-blue text-rayhunter-blue bg-white disabled:opacity-50"
                onclick={check_again}
                disabled={checking}
            >
                {checking ? 'Checking...' : 'Check again'}
            </button>
            {#if check_result === 'still_unreachable'}
                <p class="text-xs text-yellow-700 mt-2">
                    Still not reachable. Make sure you're connected to the hotspot WiFi.
                </p>
            {/if}
        </div>
    {/if}

    {#if verified || check_result === 'ok'}
        <div class="text-left text-sm text-gray-600 bg-gray-50 rounded-lg p-4 space-y-2">
            <p class="font-semibold text-gray-800">What's next</p>
            {#if is_usb_only}
                <ul class="list-disc list-inside space-y-1">
                    <li>Recordings are stored on the device.</li>
                    <li>Reconnect over USB or your phone's WiFi to download them.</li>
                    <li>
                        See the
                        <a
                            class="text-rayhunter-blue underline"
                            href="https://efforg.github.io/rayhunter/"
                            target="_blank"
                            rel="noreferrer">documentation</a
                        >
                        for device-specific notes.
                    </li>
                </ul>
            {:else}
                <ul class="list-disc list-inside space-y-1">
                    <li>
                        Visit
                        <a
                            class="text-rayhunter-blue underline"
                            href="http://{admin_ip}:8080"
                            target="_blank"
                            rel="noreferrer">http://{admin_ip}:8080</a
                        >
                        to view recordings.
                    </li>
                    <li>Configure notifications and analyzers from the web interface.</li>
                    <li>
                        Read the
                        <a
                            class="text-rayhunter-blue underline"
                            href="https://efforg.github.io/rayhunter/"
                            target="_blank"
                            rel="noreferrer">full documentation</a
                        > for usage and tips.
                    </li>
                </ul>
            {/if}
        </div>
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
