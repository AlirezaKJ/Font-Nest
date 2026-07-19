/**
 * Minimal WCAG 2.x relative-luminance and contrast helpers plus a small CSS
 * custom-property reader. Used by the token contrast test to keep every
 * foreground/surface pairing at or above the AA threshold in both themes.
 *
 * Reference: https://www.w3.org/TR/WCAG22/#dfn-relative-luminance
 */

export type Rgb = { r: number; g: number; b: number };

/** Parse a 6-digit `#rrggbb` string. Returns null for anything else. */
export function parseHex(hex: string | undefined): Rgb | null {
	if (!hex) return null;
	const match = /^#([0-9a-f]{6})$/i.exec(hex.trim());
	if (!match) return null;
	const value = Number.parseInt(match[1], 16);
	return { r: (value >> 16) & 0xff, g: (value >> 8) & 0xff, b: value & 0xff };
}

function linearize(channel: number): number {
	const c = channel / 255;
	return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

export function relativeLuminance({ r, g, b }: Rgb): number {
	return 0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b);
}

/** WCAG contrast ratio between two colors, always >= 1. */
export function contrastRatio(a: Rgb, b: Rgb): number {
	const la = relativeLuminance(a);
	const lb = relativeLuminance(b);
	const lighter = Math.max(la, lb);
	const darker = Math.min(la, lb);
	return (lighter + 0.05) / (darker + 0.05);
}

/**
 * Pull `--color-*: #rrggbb;` declarations out of the first rule matched by
 * `selector`. Rules in this stylesheet are flat (no nested braces), so the
 * capture group stops at the first closing brace. Non-hex values (rgb(),
 * none, ...) are skipped.
 */
export function extractColorTokens(css: string, selector: RegExp): Record<string, string> {
	const block = selector.exec(css);
	if (!block) return {};
	const tokens: Record<string, string> = {};
	const declaration = /(--color-[\w-]+):\s*(#[0-9a-fA-F]{6})\s*;/g;
	let match: RegExpExecArray | null;
	while ((match = declaration.exec(block[1])) !== null) {
		tokens[match[1]] = match[2];
	}
	return tokens;
}
