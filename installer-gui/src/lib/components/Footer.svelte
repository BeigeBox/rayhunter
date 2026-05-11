<script lang="ts">
    import { getVersion } from '@tauri-apps/api/app';

    let version = $state<string | null>(null);

    $effect(() => {
        getVersion().then((v) => {
            version = v;
        });
    });
</script>

<footer
    class="px-4 py-2 bg-gray-100 border-t border-gray-200 text-xs text-gray-500 flex justify-between"
>
    <span>v{version ?? '...'}</span>
</footer>
