<script lang="ts">
    import { openUrl } from '@tauri-apps/plugin-opener';

    let {
        url,
        oncancel,
    }: {
        url: string;
        oncancel: () => void;
    } = $props();

    let opened = $state(false);
    let confirming = $state(false);
    let overlay_el: HTMLDivElement | undefined = $state();
    let previous_focus: HTMLElement | null = null;

    async function open_browser() {
        try {
            await openUrl(url);
            opened = true;
        } catch {
            // Opener may be allow-listed only to specific URLs; fall back to a
            // window.open so the user can still reach the local install server.
            window.open(url, '_blank');
            opened = true;
        }
    }

    function request_cancel() {
        confirming = true;
    }

    function confirm_cancel() {
        confirming = false;
        oncancel();
    }

    function keep_installing() {
        confirming = false;
    }

    function handle_keydown(event: KeyboardEvent) {
        if (event.key === 'Escape') {
            event.preventDefault();
            if (confirming) {
                keep_installing();
            } else {
                request_cancel();
            }
            return;
        }
        if (event.key === 'Tab' && overlay_el) {
            const focusables = overlay_el.querySelectorAll<HTMLElement>(
                'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
            );
            if (focusables.length === 0) return;
            const first = focusables[0];
            const last = focusables[focusables.length - 1];
            if (event.shiftKey && document.activeElement === first) {
                event.preventDefault();
                last.focus();
            } else if (!event.shiftKey && document.activeElement === last) {
                event.preventDefault();
                first.focus();
            }
        }
    }

    $effect(() => {
        previous_focus = document.activeElement as HTMLElement | null;
        if (overlay_el) {
            const focusable = overlay_el.querySelector<HTMLElement>('button:not([disabled])');
            focusable?.focus();
        }
        return () => {
            previous_focus?.focus();
        };
    });
</script>

<svelte:window onkeydown={handle_keydown} />

<div
    class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
    role="dialog"
    aria-modal="true"
    aria-labelledby="tplink-overlay-title"
    bind:this={overlay_el}
>
    <div class="bg-white rounded-xl shadow-xl max-w-sm mx-4 p-6 space-y-4">
        {#if confirming}
            <h3 id="tplink-overlay-title" class="text-lg font-semibold">Cancel installation?</h3>
            <p class="text-sm text-gray-600">
                The device may be left in a partial state. Some steps can't be safely interrupted,
                so cancellation may take a moment to take effect.
            </p>
            <div class="flex justify-end gap-2">
                <button
                    type="button"
                    class="cursor-pointer px-4 py-2 rounded-lg border border-gray-300 text-gray-700 bg-white"
                    onclick={keep_installing}
                >
                    Keep installing
                </button>
                <button
                    type="button"
                    class="cursor-pointer px-4 py-2 rounded-lg shadow-md bg-red-500 text-white"
                    onclick={confirm_cancel}
                >
                    Cancel install
                </button>
            </div>
        {:else}
            <h3 id="tplink-overlay-title" class="text-lg font-semibold">
                Action needed: Log into your TP-Link
            </h3>
            <p class="text-sm text-gray-600">
                The TP-Link firmware uses single-use authentication tokens that can't be replayed
                from the installer. The installer is running a local helper at the address below
                that will capture the token after you log in through a browser.
            </p>
            <p class="text-sm text-gray-600">
                Open the page below, then log into your TP-Link admin panel. The installation will
                continue automatically.
            </p>
            <div class="flex flex-col gap-2">
                <button
                    type="button"
                    class="cursor-pointer w-full px-5 py-2 rounded-lg shadow-md bg-rayhunter-blue text-white"
                    onclick={open_browser}
                >
                    {opened ? 'Open browser again' : 'Open browser'}
                </button>
                <p class="text-xs text-center text-gray-500 break-all select-text">{url}</p>
            </div>
            {#if opened}
                <div class="flex items-center justify-center gap-2 text-sm text-gray-500">
                    <div
                        class="w-4 h-4 border-2 border-gray-400 border-t-transparent rounded-full animate-spin"
                    ></div>
                    Waiting for login...
                </div>
            {/if}
            <div class="flex justify-end">
                <button
                    type="button"
                    class="cursor-pointer px-4 py-2 rounded-lg border border-red-500 text-red-500 text-sm"
                    onclick={request_cancel}
                >
                    Cancel install
                </button>
            </div>
        {/if}
    </div>
</div>
