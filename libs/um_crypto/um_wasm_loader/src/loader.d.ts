export * from '../pkg/um_wasm';

/**
 * Get package version.
 * @returns {string}
 */
export function getUmcVersion(): string;

export const ready: Promise<void>;
