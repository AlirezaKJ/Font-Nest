import type { IconName } from '$lib/components/Icon.svelte';

export type ContextMenuAction = {
	kind: 'action';
	id: string;
	label: string;
	icon?: IconName;
	/** Right-aligned qualifier: a keyboard shortcut, a value, or a short note. */
	hint?: string;
	disabled?: boolean;
	run: () => void | Promise<void>;
};

export type ContextMenuSeparator = { kind: 'separator'; id: string };

export type ContextMenuEntry = ContextMenuAction | ContextMenuSeparator;

export type ContextMenuRequest = {
	/** Names what was right-clicked, shown as a header above the actions. */
	title?: string;
	/** Secondary line under the title, for the kind of thing the title names. */
	subtitle?: string;
	entries: ContextMenuEntry[];
};

/**
 * Produces the menu for one element. Returning `null` declines the event, which lets it
 * bubble to an outer provider or to the app-wide fallback menu.
 */
export type ContextMenuBuilder = (event: MouseEvent) => ContextMenuRequest | null;

export function isAction(entry: ContextMenuEntry): entry is ContextMenuAction {
	return entry.kind === 'action';
}

/**
 * Drops separators that would render as a leading, trailing, or doubled rule. Builders can
 * then add a separator after every optional group without tracking what came before.
 */
export function tidyEntries(entries: ContextMenuEntry[]): ContextMenuEntry[] {
	const tidied: ContextMenuEntry[] = [];
	for (const entry of entries) {
		if (entry.kind === 'separator' && tidied.at(-1)?.kind !== 'action') continue;
		tidied.push(entry);
	}
	while (tidied.at(-1)?.kind === 'separator') tidied.pop();
	return tidied;
}
