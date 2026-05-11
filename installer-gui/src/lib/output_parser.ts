import type { StepDefinition, InstallStep } from './types';

export type OverlayAction = { type: 'tplink_browser'; url: string };

export interface ParseResult {
    steps: InstallStep[];
    overlay_action?: OverlayAction | null;
}

// eslint-disable-next-line no-control-regex
const ANSI_RE = /\x1b\[[0-9;]*m/g;
const TPLINK_LISTENING_RE = /^Listening on (https?:\/\/\S+)/;

function clean_line(line: string): string {
    return line.replace(ANSI_RE, '').replace(/\r+$/, '');
}

function detect_overlay(cleaned: string): OverlayAction | null {
    const m = cleaned.match(TPLINK_LISTENING_RE);
    if (m) return { type: 'tplink_browser', url: m[1] };
    return null;
}

/**
 * Match a marker against a cleaned line. Markers must appear at the
 * start of the line (after ANSI strip and \r trim) so that hostile
 * device output containing a marker substring can't advance the UI.
 */
function line_matches_marker(cleaned: string, marker: string): boolean {
    return cleaned.startsWith(marker);
}

/**
 * Pure single-line parser. Caller is responsible for splitting input
 * into complete lines. Use create_output_parser for streamed input
 * where a marker may straddle two emits.
 */
export function parse_output_line(
    line: string,
    steps: InstallStep[],
    step_defs: StepDefinition[]
): ParseResult {
    const cleaned = clean_line(line);
    const active_index = steps.findIndex((s) => s.status === 'active');
    const action = detect_overlay(cleaned);

    for (let i = active_index + 1; i < step_defs.length; i++) {
        const marker = step_defs[i].marker;
        if (marker !== null && line_matches_marker(cleaned, marker)) {
            const new_steps = steps.map((s, j) => {
                if (j < i) return { ...s, status: 'done' as const };
                if (j === i) return { ...s, status: 'active' as const };
                return s;
            });
            return { steps: new_steps, overlay_action: action };
        }
    }

    return { steps, overlay_action: action };
}

/**
 * Stateful parser that buffers partial lines across calls. The installer
 * emits some progress with print! (no trailing newline); a marker split
 * across two events would be missed by parse_output_line alone.
 */
export interface OutputParser {
    feed(text: string, steps: InstallStep[], step_defs: StepDefinition[]): ParseResult;
    flush(steps: InstallStep[], step_defs: StepDefinition[]): ParseResult;
}

export function create_output_parser(): OutputParser {
    let pending = '';

    function feed(text: string, steps: InstallStep[], step_defs: StepDefinition[]): ParseResult {
        let cur_steps = steps;
        let last_action: OverlayAction | null = null;

        pending += text;
        let nl: number;
        while ((nl = pending.indexOf('\n')) !== -1) {
            const line = pending.slice(0, nl);
            pending = pending.slice(nl + 1);
            if (!line.trim()) continue;
            const r = parse_output_line(line, cur_steps, step_defs);
            cur_steps = r.steps;
            if (r.overlay_action) last_action = r.overlay_action;
        }

        return { steps: cur_steps, overlay_action: last_action };
    }

    function flush(steps: InstallStep[], step_defs: StepDefinition[]): ParseResult {
        if (!pending.trim()) {
            pending = '';
            return { steps, overlay_action: null };
        }
        const r = parse_output_line(pending, steps, step_defs);
        pending = '';
        return r;
    }

    return { feed, flush };
}

export function mark_all_done(steps: InstallStep[]): InstallStep[] {
    return steps.map((s) => ({ ...s, status: 'done' as const }));
}

export function mark_active_error(steps: InstallStep[]): InstallStep[] {
    return steps.map((s) => (s.status === 'active' ? { ...s, status: 'error' as const } : s));
}
