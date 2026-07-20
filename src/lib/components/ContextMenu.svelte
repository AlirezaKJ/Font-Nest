<script lang="ts">
	import { onMount } from 'svelte';

	import { writeClipboardText } from '$lib/context-menu/clipboard';
	import {
		clearField,
		copySelection,
		cutSelection,
		pasteIntoField,
		selectAll
	} from '$lib/context-menu/editing';
	import { placeContextMenu } from '$lib/context-menu/placement';
	import {
		activeContextMenu,
		closeContextMenu,
		openContextMenu
	} from '$lib/context-menu/store.svelte';
	import {
		currentSelectionText,
		describeEventTarget,
		describeFallbackTarget
	} from '$lib/context-menu/target';
	import {
		isAction,
		tidyEntries,
		type ContextMenuAction,
		type ContextMenuEntry,
		type ContextMenuRequest
	} from '$lib/context-menu/types';

	import Icon from './Icon.svelte';
	import type { AppView } from './AppNavigation.svelte';

	const TYPEAHEAD_RESET_MS = 600;
	const SELECTION_PREVIEW_LENGTH = 28;

	let {
		resolvedTheme,
		sidebarCollapsed,
		onToast,
		onRefresh,
		onToggleTheme,
		onToggleSidebar,
		onNavigate,
		onSearch,
		onPreviewText
	}: {
		resolvedTheme: 'light' | 'dark';
		sidebarCollapsed: boolean;
		onToast: (message: string, tone: 'success' | 'error') => void;
		onRefresh: () => void;
		onToggleTheme: () => void;
		onToggleSidebar: () => void;
		onNavigate: (view: AppView) => void;
		onSearch: (value: string) => void;
		onPreviewText: (value: string) => void;
	} = $props();

	let menuElement = $state<HTMLDivElement>();
	// Deliberately not reactive: the placement effect writes it, and reading it as state
	// would make the effect re-run itself.
	let placedToken = 0;
	let typeahead = '';
	let typeaheadTimer: ReturnType<typeof setTimeout> | undefined;
	let instantTimer: ReturnType<typeof setTimeout> | undefined;

	let menu = $derived(activeContextMenu());

	// One sliding highlight is shared by every action, matching the sidebar. Items register
	// under their entry ID so the pill can locate whichever is hovered or focused.
	let itemNodes = $state<Record<string, HTMLElement | null>>({});
	let hoveredId = $state<string | null>(null);
	let focusedId = $state<string | null>(null);
	// Geometry and visibility are separate state so measureIndicator never reads what it
	// writes (reading `indicator` inside the measuring effect would create an update loop).
	let indicator = $state({ x: 0, y: 0, width: 0, height: 0 });
	let indicatorVisible = $state(false);
	// A freshly opened menu must not slide the pill in from wherever the last one left it.
	let indicatorInstant = $state(true);

	// Hover wins over keyboard focus, as in the sidebar.
	let highlightedId = $derived(hoveredId ?? focusedId);

	function registerItem(node: HTMLElement, id: string) {
		let currentId = id;
		itemNodes[currentId] = node;
		return {
			update(nextId: string) {
				if (nextId === currentId) return;
				itemNodes[currentId] = null;
				currentId = nextId;
				itemNodes[currentId] = node;
			},
			destroy() {
				itemNodes[currentId] = null;
			}
		};
	}

	/**
	 * The entry an element belongs to.
	 *
	 * Focus is tracked with one delegated `focusin` on the menu rather than a handler per
	 * button, because keyboard navigation moves focus programmatically and the non-bubbling
	 * `focus` event does not reliably reach a per-item handler.
	 */
	function entryIdOf(target: EventTarget | null): string | null {
		if (!(target instanceof Element)) return null;
		return target.closest<HTMLElement>('[data-entry-id]')?.dataset.entryId ?? null;
	}

	// Offsets, not bounding rects: the menu scales during its entrance animation, which
	// would skew any measurement taken from getBoundingClientRect mid-flight. offsetTop and
	// offsetLeft are untransformed and already relative to the scrolling padding box, which
	// is the same box the absolutely positioned pill lives in.
	function measureIndicator() {
		const node = highlightedId ? itemNodes[highlightedId] : null;
		if (!menuElement || !node || node.offsetParent === null) {
			indicatorVisible = false;
			return;
		}

		indicator = {
			x: node.offsetLeft,
			y: node.offsetTop,
			width: node.offsetWidth,
			height: node.offsetHeight
		};
		indicatorVisible = true;
	}

	$effect(() => {
		// Re-measure when the target changes, and when a new menu swaps the item set out.
		void highlightedId;
		void menu;
		measureIndicator();
	});

	let indicatorStyle = $derived(
		`transform: translate(${indicator.x}px, ${indicator.y}px);` +
			` width: ${indicator.width}px; height: ${indicator.height}px;` +
			` opacity: ${indicatorVisible ? 1 : 0};`
	);

	onMount(() => {
		// Capture phase, so the platform menu is suppressed everywhere before any element
		// decides whether it wants to show one of ours. Modal dialogs are the exception:
		// they make the rest of the document inert, which would swallow our menu too, so
		// the platform menu stays available inside them.
		const suppressNativeMenu = (event: MouseEvent) => {
			if (!isInsideModalDialog(event.target)) event.preventDefault();
		};
		// Bubble phase. Element-level providers stop propagation, so anything that reaches
		// the window was not claimed and gets the app-wide menu instead.
		const handleUnclaimedMenu = (event: MouseEvent) => {
			if (isInsideModalDialog(event.target)) return;
			const request = buildFallbackMenu(event);
			if (!request) return;
			openContextMenu(request, { x: event.clientX, y: event.clientY });
		};
		const handleKeydown = (event: KeyboardEvent) => {
			if (event.key !== 'ContextMenu' && !(event.key === 'F10' && event.shiftKey)) return;
			const target = document.activeElement;
			if (!(target instanceof HTMLElement)) return;
			event.preventDefault();
			const rect = target.getBoundingClientRect();
			target.dispatchEvent(
				new MouseEvent('contextmenu', {
					bubbles: true,
					cancelable: true,
					clientX: Math.round(rect.left + 12),
					clientY: Math.round(rect.top + Math.min(rect.height, 28))
				})
			);
		};
		const closeForLayoutChange = () => closeContextMenu();

		window.addEventListener('contextmenu', suppressNativeMenu, true);
		window.addEventListener('contextmenu', handleUnclaimedMenu);
		window.addEventListener('keydown', handleKeydown);
		window.addEventListener('resize', closeForLayoutChange);
		window.addEventListener('scroll', closeForLayoutChange, true);

		return () => {
			window.removeEventListener('contextmenu', suppressNativeMenu, true);
			window.removeEventListener('contextmenu', handleUnclaimedMenu);
			window.removeEventListener('keydown', handleKeydown);
			window.removeEventListener('resize', closeForLayoutChange);
			window.removeEventListener('scroll', closeForLayoutChange, true);
			if (typeaheadTimer) clearTimeout(typeaheadTimer);
			if (instantTimer) clearTimeout(instantTimer);
		};
	});

	// Show, measure, and place in one synchronous pass. Effects run after the DOM updates
	// and before the browser paints, so the menu is never visible at the wrong position.
	$effect(() => {
		const current = menu;
		const element = menuElement;
		if (!element) return;

		if (!current) {
			if (element.matches(':popover-open')) element.hidePopover();
			return;
		}
		if (current.token === placedToken) return;

		// A new menu starts with no hover and no transition, so the pill lands on the first
		// item instead of sliding across from wherever the previous menu ended.
		hoveredId = null;
		indicatorInstant = true;
		if (instantTimer) clearTimeout(instantTimer);
		instantTimer = setTimeout(() => (indicatorInstant = false));

		if (element.matches(':popover-open')) element.hidePopover();
		element.showPopover();

		// offsetWidth/Height, not getBoundingClientRect: the entrance animation scales the
		// menu, and a measured box that is still mid-scale would place it slightly wrong.
		const placement = placeContextMenu(
			current.anchor,
			{ width: element.offsetWidth, height: element.offsetHeight },
			{ width: window.innerWidth, height: window.innerHeight }
		);
		element.style.setProperty('--menu-left', `${placement.x}px`);
		element.style.setProperty('--menu-top', `${placement.y}px`);
		element.style.setProperty('--menu-max-height', `${placement.maxHeight}px`);
		element.style.setProperty('--menu-origin-x', placement.flippedHorizontally ? '100%' : '0');
		element.style.setProperty('--menu-origin-y', placement.flippedVertically ? '100%' : '0');
		placedToken = current.token;

		element.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
		// Reopening a menu can reuse the same button element, in which case focus() is a
		// no-op and fires no event. Seed the highlight directly so the pill still appears.
		focusedId = current.entries.find((entry) => isAction(entry) && !entry.disabled)?.id ?? null;
		// Measure here rather than leaving it to the effect below. Reopening the same menu
		// leaves `focusedId` unchanged, so that effect would not re-run and the pill would
		// stay hidden.
		measureIndicator();
	});

	function isInsideModalDialog(target: EventTarget | null): boolean {
		if (!(target instanceof Element)) return false;
		const dialog = target.closest('dialog[open]');
		return dialog instanceof HTMLDialogElement && dialog.matches(':modal');
	}

	function actionButtons(): HTMLButtonElement[] {
		return menuElement
			? Array.from(menuElement.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'))
			: [];
	}

	async function runAction(action: ContextMenuAction) {
		closeContextMenu();
		await action.run();
	}

	function handleMenuKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			closeContextMenu(true);
			return;
		}
		if (event.key === 'Tab') {
			event.preventDefault();
			closeContextMenu(true);
			return;
		}

		const buttons = actionButtons();
		if (!buttons.length) return;
		const currentIndex = buttons.indexOf(document.activeElement as HTMLButtonElement);

		let nextIndex: number | null = null;
		if (event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % buttons.length;
		if (event.key === 'ArrowUp') {
			nextIndex = (currentIndex - 1 + buttons.length) % buttons.length;
		}
		if (event.key === 'Home') nextIndex = 0;
		if (event.key === 'End') nextIndex = buttons.length - 1;

		if (nextIndex === null) {
			if (event.key.length === 1 && !event.metaKey && !event.ctrlKey && !event.altKey) {
				nextIndex = matchTypeahead(event.key, buttons, currentIndex);
			}
			if (nextIndex === null) return;
		}

		event.preventDefault();
		buttons[nextIndex]?.focus();
	}

	function matchTypeahead(
		key: string,
		buttons: HTMLButtonElement[],
		currentIndex: number
	): number | null {
		typeahead += key.toLowerCase();
		if (typeaheadTimer) clearTimeout(typeaheadTimer);
		typeaheadTimer = setTimeout(() => (typeahead = ''), TYPEAHEAD_RESET_MS);

		// Start one past the focused item so repeating a letter cycles through matches.
		for (let offset = 1; offset <= buttons.length; offset += 1) {
			const index = (currentIndex + offset) % buttons.length;
			const label = buttons[index]?.dataset.label?.toLowerCase() ?? '';
			if (label.startsWith(typeahead)) return index;
		}
		return null;
	}

	/** The menu shown when no element claimed the right-click. */
	function buildFallbackMenu(event: MouseEvent): ContextMenuRequest | null {
		const element = event.target instanceof HTMLElement ? event.target : null;
		const target = describeFallbackTarget(
			describeEventTarget(event.target, currentSelectionText())
		);

		if (target.kind === 'editable' && element) {
			return editableMenu(element, target.readOnly, target.hasValue, target.selectedText);
		}
		if (target.kind === 'selection') return selectionMenu(target.selectedText);
		return backgroundMenu();
	}

	function editableMenu(
		element: HTMLElement,
		readOnly: boolean,
		hasValue: boolean,
		selection: string
	): ContextMenuRequest {
		const entries: ContextMenuEntry[] = [
			{
				kind: 'action',
				id: 'cut',
				label: 'Cut',
				icon: 'cut',
				hint: 'Ctrl+X',
				disabled: readOnly || !selection,
				run: async () => {
					if (!(await cutSelection(element))) {
						onToast('FontNest could not reach the clipboard.', 'error');
					}
				}
			},
			{
				kind: 'action',
				id: 'copy',
				label: 'Copy',
				icon: 'copy',
				hint: 'Ctrl+C',
				disabled: !selection,
				run: async () => {
					if (!(await copySelection(element))) {
						onToast('FontNest could not reach the clipboard.', 'error');
					}
				}
			},
			{
				kind: 'action',
				id: 'paste',
				label: 'Paste',
				icon: 'paste',
				hint: 'Ctrl+V',
				disabled: readOnly,
				run: async () => {
					if (!(await pasteIntoField(element))) {
						onToast('Clipboard access was refused. Press Ctrl+V instead.', 'error');
					}
				}
			},
			{ kind: 'separator', id: 'edit-rule' },
			{
				kind: 'action',
				id: 'select-all',
				label: 'Select all',
				icon: 'select-all',
				hint: 'Ctrl+A',
				disabled: !hasValue,
				run: () => selectAll(element)
			},
			{
				kind: 'action',
				id: 'clear',
				label: 'Clear',
				icon: 'close',
				disabled: readOnly || !hasValue,
				run: () => clearField(element)
			}
		];

		return { title: 'Text', entries: tidyEntries(entries) };
	}

	function selectionMenu(selection: string): ContextMenuRequest {
		return {
			title: truncate(selection),
			subtitle: 'Selected text',
			entries: tidyEntries([
				{
					kind: 'action',
					id: 'copy',
					label: 'Copy',
					icon: 'copy',
					hint: 'Ctrl+C',
					run: () => void copyWithFeedback('Selection', selection)
				},
				{
					kind: 'action',
					id: 'preview-text',
					label: 'Use as preview text',
					icon: 'font',
					run: () => {
						onPreviewText(selection);
						onToast('Preview text updated.', 'success');
					}
				},
				{
					kind: 'action',
					id: 'search',
					label: 'Search your library',
					icon: 'search',
					hint: truncate(selection, 18),
					run: () => onSearch(selection)
				}
			])
		};
	}

	function backgroundMenu(): ContextMenuRequest {
		return {
			title: 'FontNest',
			entries: tidyEntries([
				{
					kind: 'action',
					id: 'refresh',
					label: 'Rescan installed fonts',
					icon: 'refresh',
					run: onRefresh
				},
				{
					kind: 'action',
					id: 'theme',
					label:
						resolvedTheme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme',
					icon: resolvedTheme === 'dark' ? 'sun' : 'moon',
					run: onToggleTheme
				},
				{
					kind: 'action',
					id: 'sidebar',
					label: sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar',
					icon: 'sidebar',
					run: onToggleSidebar
				},
				{ kind: 'separator', id: 'nav-rule' },
				{
					kind: 'action',
					id: 'library',
					label: 'Go to your fonts',
					icon: 'library',
					run: () => onNavigate('library')
				},
				{
					kind: 'action',
					id: 'settings',
					label: 'Open settings',
					icon: 'settings',
					run: () => onNavigate('settings')
				},
				{
					kind: 'action',
					id: 'whats-new',
					label: 'What’s new',
					icon: 'sparkle',
					run: () => onNavigate('whatsNew')
				}
			])
		};
	}

	async function copyWithFeedback(label: string, value: string) {
		if (await writeClipboardText(value)) onToast(`${label} copied.`, 'success');
		else onToast('FontNest could not reach the clipboard.', 'error');
	}

	function truncate(value: string, length = SELECTION_PREVIEW_LENGTH): string {
		const collapsed = value.replace(/\s+/g, ' ').trim();
		return collapsed.length > length ? `${collapsed.slice(0, length - 1)}…` : collapsed;
	}
</script>

<div
	bind:this={menuElement}
	class="context-menu"
	popover="auto"
	role="menu"
	tabindex="-1"
	aria-label={menu?.title ?? 'Actions'}
	onkeydown={handleMenuKeydown}
	onpointerleave={() => (hoveredId = null)}
	onfocusin={(event) => (focusedId = entryIdOf(event.target))}
	onfocusout={() => (focusedId = null)}
	ontoggle={(event) => {
		if (event.newState === 'closed' && activeContextMenu()) closeContextMenu();
	}}
>
	{#if menu}
		<span
			class:instant={indicatorInstant}
			class="menu-indicator"
			style={indicatorStyle}
			aria-hidden="true"
		></span>
		{#if menu.title}
			<div class="menu-header">
				<strong>{menu.title}</strong>
				{#if menu.subtitle}<span>{menu.subtitle}</span>{/if}
			</div>
		{/if}
		{#each menu.entries as entry (entry.id)}
			{#if entry.kind === 'separator'}
				<hr aria-hidden="true" />
			{:else}
				<button
					use:registerItem={entry.id}
					type="button"
					role="menuitem"
					data-label={entry.label}
					data-entry-id={entry.id}
					disabled={entry.disabled}
					onpointerenter={() => (hoveredId = entry.disabled ? null : entry.id)}
					onclick={() => void runAction(entry)}
				>
					<span class="menu-icon">
						{#if entry.icon}<Icon name={entry.icon} size={15} />{/if}
					</span>
					<span class="menu-label">{entry.label}</span>
					{#if entry.hint}<span class="menu-hint">{entry.hint}</span>{/if}
				</button>
			{/if}
		{/each}
	{/if}
</div>

<style>
	.context-menu {
		position: fixed;
		inset: auto;
		top: var(--menu-top, 0);
		left: var(--menu-left, 0);
		z-index: var(--z-tooltip);
		width: max-content;
		min-width: 208px;
		max-width: min(340px, calc(100vw - 16px));
		max-height: var(--menu-max-height, 100dvh);
		margin: 0;
		padding: 4px;
		overflow-y: auto;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-raised);
		box-shadow: var(--shadow-floating);
	}

	.context-menu:popover-open {
		animation: menu-in 130ms cubic-bezier(0.16, 1, 0.3, 1);
		transform-origin: var(--menu-origin-x, 0) var(--menu-origin-y, 0);
	}

	.context-menu::backdrop {
		background: transparent;
	}

	.menu-header {
		display: grid;
		gap: 1px;
		padding: 7px 9px 8px;
		border-bottom: 1px solid var(--color-border);
		margin-bottom: 4px;
	}

	.menu-header strong {
		overflow: hidden;
		font-size: var(--text-label);
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.menu-header span {
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	/* The pill that slides between actions, mirroring the sidebar's shared highlight. */
	.menu-indicator {
		position: absolute;
		top: 0;
		left: 0;
		z-index: 0;
		border-radius: var(--radius-sm);
		background: var(--color-selected);
		box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-text) 6%, transparent);
		pointer-events: none;
		will-change: transform, width, height;
		transition:
			transform var(--motion-standard),
			width var(--motion-standard),
			height var(--motion-standard),
			opacity var(--motion-fast);
	}

	/* Opening a menu parks the pill on the first item rather than animating into place. */
	.menu-indicator.instant {
		transition: none;
	}

	.context-menu button {
		position: relative;
		z-index: 1;
		display: grid;
		width: 100%;
		min-height: 32px;
		grid-template-columns: 18px minmax(0, 1fr) auto;
		align-items: center;
		gap: var(--space-sm);
		padding: 5px 9px 5px 6px;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		text-align: left;
		cursor: pointer;
		transition: color var(--motion-fast);
	}

	.context-menu button:hover:not(:disabled),
	.context-menu button:focus-visible:not(:disabled),
	.context-menu button:focus:not(:disabled) {
		color: var(--color-text);
	}

	.context-menu button:disabled {
		color: var(--color-subtle);
		opacity: 0.55;
		cursor: not-allowed;
	}

	.menu-icon {
		display: grid;
		place-items: center;
	}

	.menu-label {
		overflow: hidden;
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.menu-hint {
		max-width: 12ch;
		overflow: hidden;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	hr {
		position: relative;
		z-index: 1;
		height: 1px;
		margin: 4px 6px;
		border: 0;
		background: var(--color-border);
	}

	@keyframes menu-in {
		from {
			opacity: 0;
			transform: scale(0.96);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.context-menu:popover-open {
			animation: none;
		}

		/* The pill still marks the target, it just stops travelling to get there. */
		.menu-indicator {
			transition: opacity var(--motion-fast);
		}
	}
</style>
