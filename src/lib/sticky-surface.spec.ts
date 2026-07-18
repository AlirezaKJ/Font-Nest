import { describe, expect, it } from 'vitest';

import { isStickySurfaceElevated } from './sticky-surface';

describe('sticky control surface', () => {
	it('elevates only once the controls reach their sticky inset', () => {
		expect(isStickySurfaceElevated(118, 120)).toBe(false);
		expect(isStickySurfaceElevated(119, 120)).toBe(true);
	});

	it('accounts for a non-zero sticky inset', () => {
		expect(isStickySurfaceElevated(78, 120, 40)).toBe(false);
		expect(isStickySurfaceElevated(79, 120, 40)).toBe(true);
	});
});
