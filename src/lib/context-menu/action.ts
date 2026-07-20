import type { Action } from 'svelte/action';

import { openContextMenu } from './store.svelte';
import type { ContextMenuBuilder } from './types';

/**
 * Claims right-clicks on one element.
 *
 * The handler stops propagation, so the innermost element that returns a menu wins and
 * everything it does not claim falls through to the app-wide menu. Returning `null` from
 * the builder declines the event outright.
 */
export const contextMenu: Action<HTMLElement, ContextMenuBuilder> = (node, builder) => {
	let build = builder;

	const handleContextMenu = (event: MouseEvent) => {
		const request = build(event);
		if (!request || request.entries.length === 0) return;
		event.preventDefault();
		event.stopPropagation();
		openContextMenu(request, pointerAnchor(event, node));
	};

	node.addEventListener('contextmenu', handleContextMenu);

	return {
		update(next: ContextMenuBuilder) {
			build = next;
		},
		destroy() {
			node.removeEventListener('contextmenu', handleContextMenu);
		}
	};
};

/**
 * Keyboard-invoked menus arrive with no pointer position, so anchor them just inside the
 * focused element instead of at the origin of the viewport.
 */
function pointerAnchor(event: MouseEvent, node: HTMLElement): { x: number; y: number } {
	if (event.clientX !== 0 || event.clientY !== 0) {
		return { x: event.clientX, y: event.clientY };
	}
	const target = event.target instanceof HTMLElement ? event.target : node;
	const rect = target.getBoundingClientRect();
	return { x: rect.left + 12, y: rect.top + Math.min(rect.height, 28) };
}
