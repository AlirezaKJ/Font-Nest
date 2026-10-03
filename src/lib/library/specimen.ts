/**
 * Drawing a family the way the library's specimen rows show it.
 *
 * These were closures over the route's state, which is why they could not be tested. Each one now
 * takes what it needs, so the rules about which weight a family is actually drawn at, and how a
 * family name reaches CSS, can be checked.
 */

import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
import { fontOrigin } from '$lib/fonts/font-origin';
import { nearestWeight, weightName } from '$lib/fonts/weights';

/** How the specimen text is chosen: the family's own name, or whatever has been typed. */
export type SpecimenMode = 'names' | 'custom';

/**
 * A family name as a CSS font stack, with the characters that would end the string or the
 * declaration taken out.
 *
 * Family names come from font files, so they are not ours to trust: a name holding a quote or a
 * semicolon would otherwise close the declaration and let the rest of it be read as more CSS.
 */
export function safeFontStack(name: string): string {
	return `"${name.replace(/["\\;\n\r]/g, '')}", system-ui, sans-serif`;
}

/**
 * The weight a family is actually drawn at.
 *
 * A variable family covers a continuous range from one file, and the weights the catalogue lists
 * for it are only the named instances it ships, often just 400. Snapping to those would pin the
 * whole slider to one weight, so the asked-for weight goes straight through and the font's own
 * wght axis clamps it. A static family can only be drawn at a cut it owns.
 */
export function drawnWeight(family: FontFamilySummary, asked: number): number {
	return family.variable ? asked : nearestWeight(family.weights, asked);
}

export function familyPreviewStyle(family: FontFamilySummary, asked: number): string {
	return `font-family: ${safeFontStack(family.name)}; font-weight: ${drawnWeight(family, asked)};`;
}

/**
 * Names the weight a family is drawn at when it owns nothing at the weight the slider asks for,
 * so a row that cannot follow the slider says why rather than looking stuck.
 */
export function substituteWeightNote(family: FontFamilySummary, asked: number): string | null {
	const drawn = drawnWeight(family, asked);
	return drawn === asked ? null : `Closest cut: ${weightName(drawn)}`;
}

export function faceSpecimenStyle(
	family: FontFamilySummary,
	weight: number,
	style: string
): string {
	return `font-family: ${safeFontStack(family.name)}; font-weight: ${weight}; font-style: ${
		style === 'italic' ? 'italic' : 'normal'
	};`;
}

/** What a specimen row draws: the family's name, or the typed text once there is any. */
export function specimenText(family: FontFamilySummary, mode: SpecimenMode, typed: string): string {
	return mode === 'names' ? family.name : typed.trim() || family.name;
}

/**
 * How far a family's weights reach.
 *
 * The collapsed row already names origin, formats, style count, spacing and technology, so the
 * open panel says only what the row cannot.
 */
export function weightRange(family: FontFamilySummary): string | null {
	if (!family.weights.length) return null;
	const lowest = Math.min(...family.weights);
	const highest = Math.max(...family.weights);
	if (family.variable) return `Weights ${lowest}–${highest} (variable)`;
	return lowest === highest
		? `Weight ${weightName(lowest)} ${lowest}`
		: `Weights ${lowest}–${highest}`;
}

/** The line under a face: its file, and only the facts the family line does not already carry. */
export function faceDetail(
	family: FontFamilySummary,
	face: FontFamilySummary['faces'][number]
): string {
	const parts = [face.fileName];
	if (family.origins.length > 1) parts.push(fontOrigin(face.origin).label);
	if (face.variable) parts.push('Variable');
	return parts.join(' · ');
}
