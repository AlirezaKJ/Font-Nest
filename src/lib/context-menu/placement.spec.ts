import { describe, expect, it } from 'vitest';

import { placeContextMenu } from './placement';

const VIEWPORT = { width: 1000, height: 800 };
const MENU = { width: 220, height: 300 };

describe('context menu placement', () => {
	it('opens from the pointer when there is room below and to the right', () => {
		const placement = placeContextMenu({ x: 120, y: 90 }, MENU, VIEWPORT);

		expect(placement).toMatchObject({
			x: 120,
			y: 90,
			flippedHorizontally: false,
			flippedVertically: false
		});
	});

	it('flips across the pointer rather than sliding along the viewport edge', () => {
		const placement = placeContextMenu({ x: 960, y: 760 }, MENU, VIEWPORT);

		expect(placement.x).toBe(960 - MENU.width);
		expect(placement.y).toBe(760 - MENU.height);
		expect(placement.flippedHorizontally).toBe(true);
		expect(placement.flippedVertically).toBe(true);
	});

	it('clamps instead of flipping when neither side has room', () => {
		const placement = placeContextMenu({ x: 30, y: 40 }, MENU, { width: 240, height: 320 });

		expect(placement.flippedHorizontally).toBe(false);
		expect(placement.flippedVertically).toBe(false);
		expect(placement.x).toBe(240 - 8 - 220);
		expect(placement.y).toBe(320 - 8 - 300);
	});

	it('caps the menu height to the viewport so long menus scroll instead of overflowing', () => {
		const placement = placeContextMenu(
			{ x: 10, y: 10 },
			{ width: 220, height: 2000 },
			VIEWPORT
		);

		// The menu now fills the usable height, so it pins to the top margin.
		expect(placement.maxHeight).toBe(800 - 16);
		expect(placement.y).toBe(8);
	});
});
