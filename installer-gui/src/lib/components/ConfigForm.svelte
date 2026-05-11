<script lang="ts">
    import type { DeviceInfo, FieldConfig, InstallMode } from '../types';
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
    let installing = $state(false);
    let advanced_open = $state(false);
    let ip_errors = $state<Record<string, string>>({});
    let shown_passwords = $state<Record<string, boolean>>({});

    const regular_fields = $derived(device.fields.filter((f) => !f.advanced));
    const advanced_fields = $derived(device.fields.filter((f) => f.advanced));

    const IP_RE = /^(\d{1,3}\.){3}\d{1,3}$/;
    const PASSWORD_CAPTION = "The password used to log into the hotspot's admin webpage.";

    function is_ip_field(field: FieldConfig): boolean {
        const key = field.key.toLowerCase();
        const label = field.label.toLowerCase();
        return key.includes('ip') || label.includes('ip');
    }

    function is_required(field: FieldConfig): boolean {
        return (field as FieldConfig & { required?: boolean }).required === true;
    }

    function has_admin_ip(): boolean {
        return device.fields.some((f) => f.key === 'adminIp');
    }

    const missing_required = $derived(
        device.fields.some((f) => {
            if (!is_required(f)) return false;
            const v = field_values[f.key];
            if (f.type === 'checkbox') return false;
            return typeof v !== 'string' || v.trim() === '';
        })
    );

    const has_ip_errors = $derived(Object.values(ip_errors).some((e) => e !== ''));

    function validate_ip(field: FieldConfig) {
        const v = field_values[field.key];
        if (typeof v !== 'string') return;
        const trimmed = v.trim();
        if (!trimmed) {
            ip_errors = { ...ip_errors, [field.key]: '' };
            return;
        }
        if (!IP_RE.test(trimmed)) {
            ip_errors = {
                ...ip_errors,
                [field.key]: 'Enter a valid IPv4 address (e.g. 192.168.1.1)',
            };
        } else {
            ip_errors = { ...ip_errors, [field.key]: '' };
        }
    }

    function status_class(status: CheckStatus): string {
        if (status === 'pass') return 'text-green-600';
        if (status === 'warn') return 'text-yellow-600';
        return 'text-red-600';
    }

    function status_icon(status: CheckStatus): string {
        if (status === 'pass') return '✓';
        if (status === 'warn') return '!';
        return '✗';
    }

    function status_aria_label(status: CheckStatus): string {
        if (status === 'pass') return 'Passed';
        if (status === 'warn') return 'Warning';
        if (status === 'fail') return 'Failed';
        return 'Pending';
    }

    async function handle_action(mode: InstallMode) {
        if (missing_required || has_ip_errors) return;
        if (!has_admin_ip()) {
            installing = true;
            oninstall(mode);
            return;
        }
        checking = true;
        const admin_ip = (field_values['adminIp'] as string) || '';
        const result = await run_preflight(admin_ip, device.id);
        preflight = result;
        checking = false;

        if (result.reachable.status !== 'fail') {
            installing = true;
            oninstall(mode);
        }
    }

    function proceed_anyway(mode: InstallMode) {
        installing = true;
        oninstall(mode);
    }

    function handle_submit(event: Event) {
        event.preventDefault();
        handle_action('install');
    }
</script>

{#snippet required_marker()}
    <span class="text-red-600" aria-label="required">&nbsp;*</span>
{/snippet}

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
    {:else if field.type === 'password'}
        <div>
            <label for={field.key} class="block text-sm font-medium mb-1">
                {field.label}
                {#if is_required(field)}{@render required_marker()}{/if}
            </label>
            <div class="relative">
                <input
                    id={field.key}
                    type={shown_passwords[field.key] ? 'text' : 'password'}
                    class="w-full px-3 py-2 pr-16 rounded-lg shadow-md"
                    autocapitalize="off"
                    autocorrect="off"
                    autocomplete="off"
                    spellcheck="false"
                    aria-required={is_required(field) ? 'true' : undefined}
                    value={field_values[field.key] ?? ''}
                    oninput={(e) => {
                        field_values = { ...field_values, [field.key]: e.currentTarget.value };
                    }}
                />
                <button
                    type="button"
                    class="cursor-pointer absolute inset-y-0 right-2 flex items-center text-xs text-gray-500 hover:text-gray-700"
                    onclick={() => {
                        shown_passwords = {
                            ...shown_passwords,
                            [field.key]: !shown_passwords[field.key],
                        };
                    }}
                    aria-label={shown_passwords[field.key] ? 'Hide password' : 'Show password'}
                >
                    {shown_passwords[field.key] ? 'Hide' : 'Show'}
                </button>
            </div>
            <p class="text-xs text-gray-500 mt-1">{PASSWORD_CAPTION}</p>
        </div>
    {:else}
        <div>
            <label for={field.key} class="block text-sm font-medium mb-1">
                {field.label}
                {#if is_required(field)}{@render required_marker()}{/if}
            </label>
            <input
                id={field.key}
                type={field.type}
                class="w-full px-3 py-2 rounded-lg shadow-md"
                autocapitalize="off"
                autocorrect="off"
                spellcheck="false"
                aria-required={is_required(field) ? 'true' : undefined}
                aria-invalid={ip_errors[field.key] ? 'true' : undefined}
                value={field_values[field.key] ?? ''}
                oninput={(e) => {
                    field_values = { ...field_values, [field.key]: e.currentTarget.value };
                    if (is_ip_field(field) && ip_errors[field.key]) {
                        ip_errors = { ...ip_errors, [field.key]: '' };
                    }
                }}
                onfocusout={() => {
                    if (is_ip_field(field)) validate_ip(field);
                }}
            />
            {#if ip_errors[field.key]}
                <p class="text-xs text-red-600 mt-1">{ip_errors[field.key]}</p>
            {/if}
        </div>
    {/if}
{/snippet}

<form class="max-w-md mx-auto pt-5 px-4 space-y-4" onsubmit={handle_submit}>
    <div class="flex items-center gap-2 mb-2">
        <button
            type="button"
            class="cursor-pointer text-sm text-gray-500 hover:text-gray-700"
            onclick={onback}
        >
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
                type="button"
                class="cursor-pointer flex items-center gap-1.5 text-sm text-gray-400 w-full"
                onclick={() => (advanced_open = !advanced_open)}
                aria-expanded={advanced_open}
            >
                <span class="text-xs">{advanced_open ? '▼' : '▶'}</span>
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

    {#if has_admin_ip() && preflight}
        <div class="space-y-1 text-sm">
            <div class={status_class(preflight.reachable.status)}>
                <span class="font-bold" aria-label={status_aria_label(preflight.reachable.status)}>
                    {status_icon(preflight.reachable.status)}
                </span>
                {preflight.reachable.label}
            </div>
        </div>
    {/if}

    <div class="flex flex-wrap gap-3 justify-center pt-2">
        {#if preflight && preflight.reachable.status === 'fail'}
            <button
                type="submit"
                class="cursor-pointer px-5 py-2 rounded-lg shadow-md bg-rayhunter-blue text-white disabled:opacity-50 disabled:cursor-not-allowed"
                disabled={checking || installing || missing_required || has_ip_errors}
            >
                Retry checks
            </button>
            <button
                type="button"
                class="cursor-pointer px-5 py-2 rounded-lg shadow-md border border-rayhunter-blue text-rayhunter-blue bg-white disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={() => proceed_anyway('install')}
                disabled={installing || missing_required || has_ip_errors}
            >
                Install anyway
            </button>
            <button
                type="button"
                class="cursor-pointer px-5 py-2 rounded-lg shadow-md border border-gray-300 text-gray-700 bg-white disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={() => proceed_anyway('update')}
                disabled={installing || missing_required || has_ip_errors}
            >
                Update anyway
            </button>
        {:else}
            <button
                type="submit"
                class="cursor-pointer px-5 py-2 rounded-lg shadow-md bg-rayhunter-blue text-white disabled:opacity-50 disabled:cursor-not-allowed"
                disabled={checking || installing || missing_required || has_ip_errors}
            >
                {#if checking}
                    Checking...
                {:else if installing}
                    Starting...
                {:else}
                    Install
                {/if}
            </button>
            <button
                type="button"
                class="cursor-pointer px-5 py-2 rounded-lg shadow-md border border-rayhunter-blue text-rayhunter-blue bg-white disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={() => handle_action('update')}
                disabled={checking || installing || missing_required || has_ip_errors}
            >
                Update existing
            </button>
        {/if}
    </div>
    <p class="text-xs text-gray-500 text-center pt-1">
        Update keeps your existing Rayhunter config; Install replaces it with defaults.
    </p>
</form>
