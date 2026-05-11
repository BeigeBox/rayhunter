<script lang="ts">
    import type { DeviceInfo, ErrorGuidance, InstallMode } from '../types';
    import { openUrl } from '@tauri-apps/plugin-opener';

    let {
        device,
        mode,
        error,
        log,
        retry_count,
        onretry,
        oncopylog,
    }: {
        device: DeviceInfo;
        mode: InstallMode;
        error: ErrorGuidance;
        log: string;
        retry_count: number;
        onretry: () => void;
        oncopylog: () => void;
    } = $props();

    let copy_label = $state('Copy log');

    const MAX_BODY_LEN = 6000;

    function redact(text: string): string {
        return text
            .replace(/--admin-password\s+\S+/gi, '--admin-password ***')
            .replace(/--password\s+\S+/gi, '--password ***')
            .replace(/--token\s+\S+/gi, '--token ***');
    }

    function tail(text: string, max: number): string {
        if (text.length <= max) return text;
        return '... (truncated) ...\n' + text.slice(text.length - max);
    }

    const issue_url = $derived.by(() => {
        const title = `Installer failure [${device.id}]`;
        const redacted_log = tail(redact(log), MAX_BODY_LEN - 400);
        const body =
            `**Device:** ${device.label} (${device.id})\n` +
            `**Mode:** ${mode}\n` +
            `**Error:** ${error.title}\n\n` +
            `<details><summary>Installer log</summary>\n\n` +
            '```\n' +
            redacted_log +
            '\n```\n\n</details>\n';
        const params = new URLSearchParams({ title, body });
        return `https://github.com/EFForg/rayhunter/issues/new?${params.toString()}`;
    });

    const repeated_failure = $derived(retry_count >= 2);

    const heading = $derived(
        error.title === 'Installation cancelled' ? 'Installation cancelled' : 'Installation failed'
    );
</script>

<div class="max-w-md mx-auto pt-12 px-4 space-y-4 text-center">
    <div
        class="w-16 h-16 mx-auto rounded-full bg-red-500 text-white flex items-center justify-center text-2xl font-bold"
        aria-hidden="true"
    >
        &times;
    </div>
    <h2 class="text-2xl font-bold">{heading}</h2>
    <div class="bg-red-50 border border-red-200 rounded-lg p-4 text-left" role="alert">
        <p class="font-semibold text-red-600">{error.title}</p>
        <p class="text-sm text-gray-700 mt-1">{error.message}</p>
    </div>
    {#if repeated_failure}
        <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-4 text-left">
            <p class="text-sm text-yellow-800">
                This keeps failing the same way. The hardware or network setup may need
                investigation. Please file an issue with the log attached so we can help.
            </p>
        </div>
    {/if}
    <div class="flex flex-wrap gap-3 justify-center pt-2">
        <button
            class="cursor-pointer px-5 py-2 rounded-lg shadow-md bg-rayhunter-blue text-white"
            onclick={onretry}
        >
            Retry
        </button>
        <button
            class="cursor-pointer px-5 py-2 rounded-lg shadow-md border border-gray-300 text-gray-700 bg-white"
            onclick={() => {
                oncopylog();
                copy_label = 'Copied!';
                setTimeout(() => {
                    copy_label = 'Copy log';
                }, 2000);
            }}
        >
            {copy_label}
        </button>
        <button
            type="button"
            onclick={() => openUrl(issue_url)}
            class="px-5 py-2 rounded-lg shadow-md border border-gray-300 text-gray-700 bg-white"
        >
            Report issue
        </button>
    </div>
</div>
