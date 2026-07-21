/**
 * Works out which file of a Google Fonts family to preview at a given weight.
 *
 * Discover downloads one file per family, so a weight control has to answer a question per
 * technology. A variable file that carries a `wght` axis covers the whole range on its own and
 * the browser interpolates it. Everything else ships one file per weight, so the family's
 * artifact list is searched for the closest upright cut and that file is fetched instead.
 *
 * The manifest describes each artifact only by a style string ("Regular", "SemiBold",
 * "Variable (opsz,wght)"), so those strings are what this module reads.
 */

export type PreviewArtifactLike = {
	id: string;
	style: string;
};

export type PreviewSelection = {
	artifactId: string;
	/** The weight the chosen file actually draws, or `null` when the file covers a range. */
	weight: number | null;
	/** True when the browser can move this file through the requested weight itself. */
	variable: boolean;
};

const STATIC_WEIGHTS: Record<string, number> = {
	thin: 100,
	hairline: 100,
	extralight: 200,
	ultralight: 200,
	light: 300,
	regular: 400,
	normal: 400,
	book: 400,
	medium: 500,
	semibold: 600,
	demibold: 600,
	bold: 700,
	extrabold: 800,
	ultrabold: 800,
	black: 900,
	heavy: 900
};

const VARIABLE_PREFIX = 'variable';

/** True when the style names a variable file, whatever axes it carries. */
export function isVariableStyle(style: string): boolean {
	return style.trim().toLowerCase().startsWith(VARIABLE_PREFIX);
}

/** True when the style names an italic file. Weight previews stay upright. */
export function isItalicStyle(style: string): boolean {
	return /italic/i.test(style);
}

/**
 * True when a variable file can be driven by CSS `font-weight`, meaning it declares a `wght`
 * axis. A variable file with only optical size or a decorative axis cannot, so it is treated
 * as a fixed weight.
 */
export function hasWeightAxis(style: string): boolean {
	if (!isVariableStyle(style)) return false;
	const open = style.indexOf('(');
	const close = style.lastIndexOf(')');
	// A style that names no axes at all says nothing either way; weight is the axis nearly
	// every variable font carries, so it gets the benefit of the doubt.
	if (open < 0 || close < open) return true;
	return style
		.slice(open + 1, close)
		.split(',')
		.map((axis) => axis.trim())
		.includes('wght');
}

/**
 * The weight a static file draws, read from its style name, or `null` when the name says
 * nothing about weight. Vendor prefixes ("Web Bold", "Caption Web Regular") are common in the
 * manifest, so the name is matched from its end.
 */
export function staticStyleWeight(style: string): number | null {
	if (isVariableStyle(style) || isItalicStyle(style)) return null;
	const condensed = style.toLowerCase().replaceAll(/[\s_-]/g, '');
	for (const [name, weight] of Object.entries(STATIC_WEIGHTS)) {
		if (condensed.endsWith(name)) return weight;
	}
	return null;
}

/**
 * Picks the file to preview `weight` with, or `null` when the family offers nothing upright.
 *
 * A variable file with a `wght` axis wins outright: one download covers every weight. Failing
 * that, the closest static weight wins, and ties go to the heavier file so a request for 550
 * from a family with 500 and 600 does not quietly read lighter than asked.
 */
export function pickPreviewArtifact(
	artifacts: readonly PreviewArtifactLike[],
	weight: number
): PreviewSelection | null {
	const upright = artifacts.filter((artifact) => !isItalicStyle(artifact.style));

	const variable = upright.find((artifact) => hasWeightAxis(artifact.style));
	if (variable) {
		return { artifactId: variable.id, weight: null, variable: true };
	}

	let best: PreviewSelection | null = null;
	let bestDistance = Number.POSITIVE_INFINITY;
	for (const artifact of upright) {
		const candidate = staticStyleWeight(artifact.style);
		if (candidate === null) continue;
		const distance = Math.abs(candidate - weight);
		if (
			distance < bestDistance ||
			(distance === bestDistance && candidate > (best?.weight ?? 0))
		) {
			best = { artifactId: artifact.id, weight: candidate, variable: false };
			bestDistance = distance;
		}
	}

	return best;
}
