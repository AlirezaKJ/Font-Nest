import type { FontOrigin } from '$lib/bindings/FontOrigin';

export type FontOriginCopy = {
	/** Short enough to sit inline in a dense list row. */
	label: string;
	/** Full sentence for tooltips, detail panels, and screen readers. */
	description: string;
};

/** Ordered the way the backend orders them: what the system owns first. */
export const FONT_ORIGIN_ORDER: readonly FontOrigin[] = [
	'systemDefault',
	'machineInstalled',
	'userInstalled',
	'unknown'
];

const ORIGIN_COPY: Record<FontOrigin, FontOriginCopy> = {
	systemDefault: {
		label: 'System font',
		description: 'Shipped with your operating system.'
	},
	machineInstalled: {
		label: 'All users',
		description: 'Installed on this computer for every account.'
	},
	userInstalled: {
		label: 'Installed',
		description: 'Installed for your account only.'
	},
	unknown: {
		label: 'Unknown source',
		description: 'FontNest could not tell where this font came from.'
	}
};

const MIXED_ORIGIN: FontOriginCopy = {
	label: 'Mixed sources',
	description: 'This family combines fonts from more than one place.'
};

export function fontOrigin(origin: FontOrigin): FontOriginCopy {
	return ORIGIN_COPY[origin] ?? ORIGIN_COPY.unknown;
}

export function fontOriginLabel(origin: FontOrigin): string {
	return fontOrigin(origin).label;
}

/**
 * Collapses a family's face origins into one chip. A family with faces from several
 * places is called mixed rather than assigned to whichever place happens to be first,
 * so nothing that includes a system font ever reads as purely installed.
 */
export function familyOrigin(origins: readonly FontOrigin[]): FontOriginCopy {
	if (origins.length === 1) return fontOrigin(origins[0]);
	if (origins.length === 0) return fontOrigin('unknown');
	return {
		label: MIXED_ORIGIN.label,
		description: `${MIXED_ORIGIN.description} ${sortOrigins(origins)
			.map((origin) => fontOriginLabel(origin))
			.join(', ')}.`
	};
}

/** True when a family contains anything the operating system provided. */
export function includesSystemFont(origins: readonly FontOrigin[]): boolean {
	return origins.includes('systemDefault');
}

/**
 * True when every face shipped with the operating system. Most of a library is this, so
 * the interface leans on it to give the fonts somebody actually added the emphasis.
 */
export function isSystemOnly(origins: readonly FontOrigin[]): boolean {
	return origins.length === 1 && origins[0] === 'systemDefault';
}

export function sortOrigins(origins: readonly FontOrigin[]): FontOrigin[] {
	return [...origins].sort(
		(left, right) => FONT_ORIGIN_ORDER.indexOf(left) - FONT_ORIGIN_ORDER.indexOf(right)
	);
}
