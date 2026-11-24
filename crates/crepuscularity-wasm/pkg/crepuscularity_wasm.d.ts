/* tslint:disable */
/* eslint-disable */

/**
 * The shared Rust engine for browser, extension and worker frame loops.
 */
export class MotionValue {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Milliseconds use the same time origin as subsequent tick calls (performance.now()).
     */
    animateTo(target: number, duration_ms: number, easing: string, now_ms: number, reduced_motion: boolean): number;
    cancel(): number;
    constructor(value: number);
    tick(now_ms: number, reduced_motion: boolean): number;
    readonly active: boolean;
}

/**
 * Schema version of the IR this module emits; consumers should check it.
 */
export function ir_version(): number;

/**
 * Parse `.crepus` source into View IR, serialized as JSON.
 *
 * `context_json`, when present, must be a JSON object whose values are bound
 * as template variables.
 */
export function parse_crepus_json(source: string, context_json?: string | null): string;

/**
 * Parse a template into View IR, choosing the frontend from `filename`.
 *
 * The parser dispatches on the file extension, so `.crepus`, `.jsx`/`.tsx`,
 * `.svelte`, `.vue`, `.astro` and Angular component templates
 * (`*.component.html`, `*.ng.html`, `*.ng`) all reach the same IR through
 * their own frontend.
 */
export function parse_template_json(source: string, filename?: string | null, context_json?: string | null): string;
