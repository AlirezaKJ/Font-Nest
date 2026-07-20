import type { ContextMenuRequest } from './types';

export type OpenContextMenu = ContextMenuRequest & {
	/** Pointer position the menu opens from, in viewport coordinates. */
	anchor: { x: number; y: number };
	/** Focus returns here on close, so keyboard users never lose their place. */
	returnFocus: HTMLElement | null;
	/** Changes on every open so the renderer can restart placement and motion. */
	token: number;
};

const state = $state<{ menu: OpenContextMenu | null }>({ menu: null });

let nextToken = 0;

export function activeContextMenu(): OpenContextMenu | null {
	return state.menu;
}

export function openContextMenu(request: ContextMenuRequest, anchor: { x: number; y: number }) {
	if (request.entries.length === 0) return;
	const active = document.activeElement;
	nextToken += 1;
	state.menu = {
		...request,
		anchor,
		returnFocus: active instanceof HTMLElement && active !== document.body ? active : null,
		token: nextToken
	};
}

export function closeContextMenu(restoreFocus = false) {
	const closing = state.menu;
	state.menu = null;
	if (restoreFocus) closing?.returnFocus?.focus();
}
