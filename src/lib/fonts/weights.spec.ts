import { describe, expect, it } from 'vitest';

import { nearestWeight, nearestWeightName, weightName } from './weights';

describe('naming a weight a font declares', () => {
	it('names the standard weights', () => {
		expect(weightName(400)).toBe('Regular');
		expect(weightName(700)).toBe('Bold');
		expect(weightName(100)).toBe('Thin');
	});

	// The number came from the font. Calling a declared 650 "Semibold" would be the interface
	// disagreeing with the file in front of it.
	it('leaves a weight the font declares that has no standard name as the number', () => {
		expect(weightName(650)).toBe('650');
		expect(weightName(1)).toBe('1');
	});
});

describe('naming the closest standard weight', () => {
	// Here the number came from a control, so a name a person recognizes is the point.
	it('rounds to the nearest standard name', () => {
		expect(nearestWeightName(650)).toBe('Semibold');
		expect(nearestWeightName(640)).toBe('Semibold');
		expect(nearestWeightName(410)).toBe('Regular');
		expect(nearestWeightName(1000)).toBe('Black');
		expect(nearestWeightName(1)).toBe('Thin');
	});

	it('names an exact standard weight as itself', () => {
		expect(nearestWeightName(500)).toBe('Medium');
	});
});

describe('choosing a weight a family owns', () => {
	it('lands on the closest cut a static family ships', () => {
		expect(nearestWeight([400, 700], 650)).toBe(700);
		expect(nearestWeight([400, 700], 500)).toBe(400);
		expect(nearestWeight([300], 900)).toBe(300);
	});

	it('falls back to Regular when a family lists no weights at all', () => {
		expect(nearestWeight([], 650)).toBe(400);
	});

	// Equal distance keeps the first cut rather than drifting heavier, so the same slider
	// position always draws the same face.
	it('settles a tie the same way every time', () => {
		expect(nearestWeight([400, 600], 500)).toBe(400);
		expect(nearestWeight([600, 400], 500)).toBe(600);
	});
});
