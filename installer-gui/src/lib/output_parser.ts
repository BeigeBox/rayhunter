import type { StepDefinition, InstallStep } from './types';

export interface ParseResult {
    steps: InstallStep[];
    overlay?: 'tplink-login' | null;
}

/**
 * Given a new output line, advance the step list forward if the line
 * matches a marker for a step after the current active step.
 * Never regresses (handles PinePhone's double "Unlocking modem").
 */
export function parse_output_line(
    line: string,
    steps: InstallStep[],
    step_defs: StepDefinition[]
): ParseResult {
    const active_index = steps.findIndex((s) => s.status === 'active');
    let overlay: 'tplink-login' | null = null;

    // TP-Link v5 overlay trigger
    if (line.includes('Listening on http://') || line.includes('Please open above URL')) {
        overlay = 'tplink-login';
    }

    // Look for a marker match in steps *after* the current active step
    for (let i = active_index + 1; i < step_defs.length; i++) {
        const marker = step_defs[i].marker;
        if (marker !== null && line.includes(marker)) {
            const new_steps = steps.map((s, j) => {
                if (j < i) return { ...s, status: 'done' as const };
                if (j === i) return { ...s, status: 'active' as const };
                return s;
            });
            return { steps: new_steps, overlay };
        }
    }

    return { steps, overlay };
}

/**
 * Mark all steps as done (for successful completion).
 */
export function mark_all_done(steps: InstallStep[]): InstallStep[] {
    return steps.map((s) => ({ ...s, status: 'done' as const }));
}

/**
 * Mark the current active step as error.
 */
export function mark_active_error(steps: InstallStep[]): InstallStep[] {
    return steps.map((s) => (s.status === 'active' ? { ...s, status: 'error' as const } : s));
}
