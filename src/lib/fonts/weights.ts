/**
 * Naming font weights, in the one place that knows the names.
 *
 * The table lived in three files, with two different ideas of what naming a weight means, and
 * both are needed: a catalogue lists the weights a family actually ships, so 650 is 650, while a
 * face drawn at 650 is fairly called Semibold. Keeping them apart as two functions says which is
 * which, instead of leaving the difference to whichever copy a file happened to hold.
 */

/** The standard weights, as the design contract and every type foundry name them. */
export const WEIGHT_NAMES: Record<number, string> = {
	100: 'Thin',
	200: 'Extralight',
	300: 'Light',
	400: 'Regular',
	500: 'Medium',
	600: 'Semibold',
	700: 'Bold',
	800: 'Extrabold',
	900: 'Black'
};

/**
 * The name of a weight a font actually declares, or the number when it is not a standard one.
 *
 * Used where the number came from the font: calling a declared 650 "Semibold" would be the
 * interface disagreeing with the file in front of it.
 */
export function weightName(weight: number): string {
	return WEIGHT_NAMES[weight] ?? String(weight);
}

/**
 * The standard name closest to a weight.
 *
 * Used where the number came from a control rather than from a font, so an axis sitting at 650
 * can be described in words a person recognizes.
 */
export function nearestWeightName(weight: number): string {
	const nearest = Object.keys(WEIGHT_NAMES)
		.map(Number)
		.reduce((closest, value) =>
			Math.abs(value - weight) < Math.abs(closest - weight) ? value : closest
		);
	return WEIGHT_NAMES[nearest] ?? 'Regular';
}

/**
 * The weight a family actually owns that sits closest to the one asked for.
 *
 * A static family cannot be drawn between its cuts, so asking for 650 has to land on one it has.
 */
export function nearestWeight(weights: number[], target: number): number {
	return weights.reduce(
		(closest, weight) =>
			Math.abs(weight - target) < Math.abs(closest - target) ? weight : closest,
		weights[0] ?? 400
	);
}
