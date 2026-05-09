<script lang="ts">
    import type { DeviceInfo, FieldConfig } from '../types';
    import { run_preflight, type PreflightResult, type CheckStatus } from '../preflight';

    let {
        device,
        field_values = $bindable(),
        oninstall,
        onback,
    }: {
        device: DeviceInfo;
        field_values: Record<string, string | boolean>;
        oninstall: (mode: 'install' | 'update') => void;
        onback: () => void;
    } = $props();

    let preflight = $state<PreflightResult | null>(null);
    let checking = $state(false);
    let advanced_open = $state(false);

    const regular_fields = $derived(device.fields.filter((f) => !f.advanced));
    const advanced_fields = $derived(device.fields.filter((f) => f.advanced));

    function status_class(status: CheckStatus): string {
        if (status === 'pass') return 'text-green-600';
        if (status === 'warn') return 'text-yellow-600';
        return 'text-red-600';
    }

    function status_icon(status: CheckStatus): string {
        if (status === 'pass') return '\u2713';
        if (status === 'warn') return '!';
        return '\u2717';
    }

    async function handle_install() {
        checking = true;
        preflight = null;
        const admin_ip = (field_values['adminIp'] as string) || '';
        const result = await run_preflight(admin_ip);
        preflight = result;
        checking = false;

        if (result.reachable.status !== 'fail') {
            oninstall('install');
        }
    }

    function proceed_anyway() {
        oninstall('install');
    }
</script>

{#snippet field_input(field: FieldConfig)}
    {#if field.type === 'checkbox'}
        <label class="flex items-center gap-2">
            <input
                type="checkbox"
                checked={field_values[field.key] === true}
                onchange={(e) => {
                    field_values = { ...field_values, [field.key]: e.currentTarget.checked };
                }}
            />
            <span class="text-sm">{field.label}</span>
        </label>
    {:else}
        <div>
            <label for={field.key} class="block text-sm font-medium mb-1">{field.label}</label>
            <input
                id={field.key}
                type={field.type}
                class="w-full px-3 py-2 rounded-lg shadow-md"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                value={field_values[field.key] ?? ''}
                oninput={(e) => {
                    field_values = { ...field_values, [field.key]: e.currentTarget.value };
                }}
            />
        </div>
    {/if}
{/snippet}

<div class="max-w-md mx-auto pt-5 px-4 space-y-4">
    <div class="flex items-center gap-2 mb-2">
        <button class="cursor-pointer text-sm text-gray-500 hover:text-gray-700" onclick={onback}>
            &larr; Back
        </button>
    </div>

    <h2 class="text-xl font-semibold text-center">{device.label}</h2>

    <hr class="border-gray-200" />

    {#each regular_fields as field (field.key)}
        {@render field_input(field)}
    {/each}

    {#if advanced_fields.length > 0}
        <div class="rounded-lg bg-gray-50 px-3 py-2">
            <button
                class="cursor-pointer flex items-center gap-1.5 text-sm text-gray-400 w-full"
                onclick={() => (advanced_open = !advanced_open)}
            >
                <span class="text-xs">{advanced_open ? '\u25BC' : '\u25B6'}</span>
                Advanced Settings
            </button>
            {#if advanced_open}
                <div class="mt-3 space-y-4">
                    {#each advanced_fields as field (field.key)}
                        {@render field_input(field)}
                    {/each}
                </div>
            {/if}
        </div>
    {/if}

    {#if preflight}
        <div class="space-y-1 text-sm">
            <div class={status_class(preflight.reachable.status)}>
                <span class="font-bold">{status_icon(preflight.reachable.status)}</span>
                {preflight.reachable.label}
            </div>
        </div>
    {/if}

    <div class="flex gap-3 justify-center pt-2">
        {#if preflight && preflight.reachable.status === 'fail'}
            <button
                class="cursor-pointer px-5 py-2 rounded-lg shadow-md bg-rayhunter-blue text-white"
                disabled={checking}
                onclick={handle_install}
            >
                Retry checks
            </button>
            <button
                class="cursor-pointer px-5 py-2 rounded-lg shadow-md border border-rayhunter-blue text-rayhunter-blue bg-white"
                onclick={proceed_anyway}
            >
                Install anyway
            </button>
        {:else}
            <button
                class="cursor-pointer px-5 py-2 rounded-lg shadow-md bg-rayhunter-blue text-white"
                disabled={checking}
                onclick={handle_install}
            >
                {checking ? 'Checking...' : 'Install'}
            </button>
        {/if}
    </div>
</div>
