export type MenuAnchor = { x: number; y: number };
export type MenuSize = { width: number; height: number };
export type Viewport = { width: number; height: number };

export type MenuPlacement = {
	x: number;
	y: number;
	maxHeight: number;
	/** True when the menu had to open above the pointer, so it can animate from below. */
	flippedVertically: boolean;
	flippedHorizontally: boolean;
};

const VIEWPORT_MARGIN = 8;

/**
 * Positions the menu at the pointer, flipping across it rather than sliding along the edge
 * so the pointer always lands on a corner and never on top of an action.
 */
export function placeContextMenu(
	anchor: MenuAnchor,
	menu: MenuSize,
	viewport: Viewport,
	margin = VIEWPORT_MARGIN
): MenuPlacement {
	const maxHeight = Math.max(0, viewport.height - margin * 2);
	const height = Math.min(menu.height, maxHeight);
	const width = Math.min(menu.width, Math.max(0, viewport.width - margin * 2));

	const overflowsRight = anchor.x + width > viewport.width - margin;
	const flippedHorizontally = overflowsRight && anchor.x - width >= margin;
	const x = clamp(
		flippedHorizontally ? anchor.x - width : anchor.x,
		margin,
		Math.max(margin, viewport.width - margin - width)
	);

	const overflowsBottom = anchor.y + height > viewport.height - margin;
	const flippedVertically = overflowsBottom && anchor.y - height >= margin;
	const y = clamp(
		flippedVertically ? anchor.y - height : anchor.y,
		margin,
		Math.max(margin, viewport.height - margin - height)
	);

	return { x, y, maxHeight, flippedVertically, flippedHorizontally };
}

function clamp(value: number, minimum: number, maximum: number): number {
	return Math.min(Math.max(value, minimum), maximum);
}
