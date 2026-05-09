<script lang="ts">
    import { openUrl } from '@tauri-apps/plugin-opener';

    let opened = $state(false);

    function open_browser() {
        openUrl('http://127.0.0.1:4000');
        opened = true;
    }
</script>

<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-white rounded-xl shadow-xl max-w-sm mx-4 p-6 space-y-4">
        <h3 class="text-lg font-semibold">Action needed: Log into your TP-Link</h3>
        <p class="text-sm text-gray-600">
            A browser window needs to be opened to complete the installation. Click the button
            below, then log into your TP-Link admin panel.
        </p>
        <p class="text-sm text-gray-600">
            The installation will continue automatically after you log in.
        </p>
        <div class="flex justify-center">
            <button
                class="cursor-pointer px-5 py-2 rounded-lg shadow-md bg-rayhunter-blue text-white"
                onclick={open_browser}
            >
                Open Browser
            </button>
        </div>
        {#if opened}
            <div class="flex items-center justify-center gap-2 text-sm text-gray-500">
                <div
                    class="w-4 h-4 border-2 border-gray-400 border-t-transparent rounded-full animate-spin"
                ></div>
                Waiting for login...
            </div>
        {/if}
    </div>
</div>
