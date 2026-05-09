<script lang="ts">
    import type { ErrorGuidance } from '../types';

    let {
        error,
        onretry,
        oncopylog,
    }: {
        error: ErrorGuidance;
        onretry: () => void;
        oncopylog: () => void;
    } = $props();

    let copy_label = $state('Copy Log');
</script>

<div class="max-w-md mx-auto pt-12 px-4 space-y-4 text-center">
    <div
        class="w-16 h-16 mx-auto rounded-full bg-red-500 text-white flex items-center justify-center text-2xl font-bold"
    >
        &times;
    </div>
    <h2 class="text-2xl font-bold">Installation failed</h2>
    <div class="bg-red-50 border border-red-200 rounded-lg p-4 text-left">
        <p class="font-semibold text-red-600">{error.title}</p>
        <p class="text-sm text-gray-700 mt-1">{error.message}</p>
        {#if error.context}
            <ul class="text-xs text-gray-500 mt-2 space-y-0.5">
                {#each error.context as line, i (i)}
                    <li>{line}</li>
                {/each}
            </ul>
        {/if}
    </div>
    <div class="flex gap-3 justify-center pt-2">
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
                    copy_label = 'Copy Log';
                }, 2000);
            }}
        >
            {copy_label}
        </button>
        <a
            href="https://github.com/EFForg/rayhunter/issues"
            target="_blank"
            class="px-5 py-2 rounded-lg shadow-md border border-gray-300 text-gray-700 bg-white"
        >
            Report Issue
        </a>
    </div>
</div>
