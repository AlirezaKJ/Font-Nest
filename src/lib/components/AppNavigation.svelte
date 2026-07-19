<script lang="ts" module>
	export type AppView =
		'library' | 'discover' | 'duplicates' | 'preview' | 'settings' | 'whatsNew';
	export type PinnedFamily = { id: string; name: string };
</script>

<script lang="ts">
	import { getDirectionalReorderPosition, type ReorderPosition } from '$lib/reorder';

	import Icon, { type IconName } from './Icon.svelte';

	let {
		view,
		familyCount,
		conflictCount,
		loading,
		collapsed,
		pinnedFamilies,
		activeFamilyId,
		unseenRelease,
		onNavigate,
		onOpenPreview,
		onClosePreview,
		onReorderPreview,
		onToggle,
		onRefresh
	}: {
		view: AppView;
		familyCount: number;
		conflictCount: number;
		loading: boolean;
		collapsed: boolean;
		pinnedFamilies: PinnedFamily[];
		activeFamilyId: string | null;
		unseenRelease: boolean;
		onNavigate: (view: AppView) => void;
		onOpenPreview: (familyId: string) => void;
		onClosePreview: (familyId: string) => void;
		onReorderPreview: (
			draggedFamilyId: string,
			targetFamilyId: string,
			position: ReorderPosition
		) => void;
		onToggle: () => void;
		onRefresh: () => void;
	} = $props();

	let draggedFamilyId = $state<string | null>(null);
	let dropTarget = $state<{ familyId: string; position: ReorderPosition } | null>(null);

	function handlePreviewDragStart(event: DragEvent, familyId: string) {
		draggedFamilyId = familyId;
		dropTarget = null;
		if (!event.dataTransfer) return;
		event.dataTransfer.effectAllowed = 'move';
		event.dataTransfer.setData('text/plain', familyId);
	}

	function handlePreviewDragOver(event: DragEvent, targetFamilyId: string) {
		if (!draggedFamilyId) {
			dropTarget = null;
			return;
		}
		const position = getDirectionalReorderPosition(
			pinnedFamilies.map((family) => family.id),
			draggedFamilyId,
			targetFamilyId
		);
		if (!position) {
			dropTarget = null;
			return;
		}

		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
		dropTarget = { familyId: targetFamilyId, position };
	}

	function handlePreviewDrop(event: DragEvent, targetFamilyId: string) {
		event.preventDefault();
		const transferredFamilyId = event.dataTransfer?.getData('text/plain') || null;
		const droppedFamilyId = draggedFamilyId ?? transferredFamilyId;
		if (droppedFamilyId) {
			const position = getDirectionalReorderPosition(
				pinnedFamilies.map((family) => family.id),
				droppedFamilyId,
				targetFamilyId
			);
			if (position) onReorderPreview(droppedFamilyId, targetFamilyId, position);
		}
		handlePreviewDragEnd();
	}

	function handlePreviewDragEnd() {
		draggedFamilyId = null;
		dropTarget = null;
	}

	type FooterAction = {
		key: string;
		icon: IconName;
		label: string;
		disabled: boolean;
		badge: boolean;
		onSelect: () => void;
	};

	let footerActions = $derived<FooterAction[]>([
		{
			key: 'refresh',
			icon: 'refresh',
			label: 'Fetch fonts',
			disabled: loading,
			badge: false,
			onSelect: onRefresh
		},
		{
			key: 'whatsNew',
			icon: 'sparkle',
			label: 'Patch notes',
			disabled: false,
			badge: unseenRelease,
			onSelect: () => onNavigate('whatsNew')
		},
		{
			key: 'settings',
			icon: 'settings',
			label: 'Settings',
			disabled: false,
			badge: false,
			onSelect: () => onNavigate('settings')
		}
	]);

	// A single sliding highlight is shared by every sidebar item — primary nav, saved previews,
	// and footer actions. Each element registers under a stable key; the pill parks on the active
	// view and follows hover / focus anywhere in the sidebar.
	let navContentElement = $state<HTMLElement>();
	let footerRowElement = $state<HTMLElement>();
	let itemNodes = $state<Record<string, HTMLElement | null>>({});
	let hoveredKey = $state<string | null>(null);
	let focusedKey = $state<string | null>(null);
	// Geometry and visibility are separate state so measureIndicator never reads what it writes
	// (reading `indicator` inside the measuring effect would create an update loop).
	let indicator = $state({ x: 0, y: 0, width: 0, height: 0 });
	let indicatorVisible = $state(false);

	let activeKey = $derived.by(() => {
		switch (view) {
			case 'library':
				return 'nav:library';
			case 'discover':
				return 'nav:discover';
			case 'duplicates':
				return 'nav:conflicts';
			case 'whatsNew':
				return 'action:whatsNew';
			case 'settings':
				return 'action:settings';
			case 'preview':
				return activeFamilyId ? `preview:${activeFamilyId}` : null;
			default:
				return null;
		}
	});
	// Hover wins over keyboard focus, which wins over the parked active view.
	let highlightedKey = $derived(hoveredKey ?? focusedKey ?? activeKey);
	let expandedActionKey = $derived(
		highlightedKey?.startsWith('action:') ? highlightedKey.slice(7) : null
	);

	// Footer geometry constants, mirrored from the CSS below.
	const FOOTER_GAP = 4;
	const FOOTER_COLLAPSED = 32;
	const FOOTER_ICON_GAP = 7;

	// Registers a node under a key so the shared indicator can locate it.
	function registerItem(node: HTMLElement, key: string) {
		let currentKey = key;
		itemNodes[currentKey] = node;
		return {
			update(nextKey: string) {
				if (nextKey === currentKey) return;
				itemNodes[currentKey] = null;
				currentKey = nextKey;
				itemNodes[currentKey] = node;
			},
			destroy() {
				itemNodes[currentKey] = null;
			}
		};
	}

	function isVisible(node: HTMLElement): boolean {
		return node.offsetParent !== null;
	}

	// The expanded footer button grows via a CSS width transition. Reading its live rect would
	// return a mid-animation width, so the pill's target is derived from the label's intrinsic
	// width instead — this lets the button animate freely while the pill lands on the final size.
	function footerGeometry(actionKey: string, content: HTMLElement, contentRect: DOMRect) {
		const row = footerRowElement;
		if (!row || !isVisible(row)) return null;
		const keys = footerActions.map((action) => action.key);
		const index = keys.indexOf(actionKey);
		if (index < 0) return null;

		const widths = keys.map((key) => {
			if (key !== expandedActionKey) return FOOTER_COLLAPSED;
			const label = itemNodes[`action:${key}`]?.querySelector<HTMLElement>('.action-label');
			return FOOTER_COLLAPSED + FOOTER_ICON_GAP + (label ? Math.ceil(label.scrollWidth) : 0);
		});
		const total =
			widths.reduce((sum, width) => sum + width, 0) + FOOTER_GAP * (keys.length - 1);
		const rowRect = row.getBoundingClientRect();
		let left = Math.max(0, (row.clientWidth - total) / 2);
		for (let i = 0; i < index; i += 1) left += widths[i] + FOOTER_GAP;

		return {
			x: rowRect.left - contentRect.left + content.scrollLeft + left,
			y: rowRect.top - contentRect.top + content.scrollTop,
			width: widths[index],
			height: FOOTER_COLLAPSED
		};
	}

	function measureIndicator() {
		const content = navContentElement;
		const key = highlightedKey;
		if (!content || !key) {
			indicatorVisible = false;
			return;
		}
		const contentRect = content.getBoundingClientRect();

		let geometry: { x: number; y: number; width: number; height: number } | null = null;
		if (key.startsWith('action:') && !collapsed) {
			geometry = footerGeometry(key.slice(7), content, contentRect);
		} else {
			const node = itemNodes[key];
			if (node && isVisible(node)) {
				const rect = node.getBoundingClientRect();
				geometry = {
					x: rect.left - contentRect.left + content.scrollLeft,
					y: rect.top - contentRect.top + content.scrollTop,
					width: rect.width,
					height: rect.height
				};
			}
		}

		if (!geometry || geometry.width === 0) {
			indicatorVisible = false;
			return;
		}
		indicator = geometry;
		indicatorVisible = true;
	}

	$effect(() => {
		// Re-measure whenever the target, layout inputs, or the item set change.
		void highlightedKey;
		void expandedActionKey;
		void collapsed;
		void footerActions;
		void pinnedFamilies;
		measureIndicator();
	});

	$effect(() => {
		const content = navContentElement;
		if (!content || typeof ResizeObserver === 'undefined') return;
		const observer = new ResizeObserver(() => measureIndicator());
		observer.observe(content);
		// Web fonts change label widths after first paint; re-measure once they settle.
		void document.fonts?.ready.then(measureIndicator);
		return () => observer.disconnect();
	});

	let indicatorStyle = $derived(
		`transform: translate(${indicator.x}px, ${indicator.y}px);` +
			` width: ${indicator.width}px; height: ${indicator.height}px;` +
			` opacity: ${indicatorVisible ? 1 : 0};`
	);
</script>

<aside
	class:collapsed
	class="app-navigation"
	aria-label="Application navigation"
	onpointerleave={() => (hoveredKey = null)}
>
	<div bind:this={navContentElement} class="navigation-content">
		<span class="sidebar-indicator" style={indicatorStyle} aria-hidden="true"></span>
		<nav aria-label="Primary">
			<p class="nav-group-label">Workspace</p>
			<button
				use:registerItem={'nav:library'}
				type="button"
				class:active={view === 'library'}
				aria-current={view === 'library' ? 'page' : undefined}
				title={collapsed ? 'Library' : undefined}
				onpointerenter={() => (hoveredKey = 'nav:library')}
				onfocus={() => (focusedKey = 'nav:library')}
				onblur={() => (focusedKey = null)}
				onclick={() => onNavigate('library')}
			>
				<Icon name="library" size={17} />
				<span class="nav-label">Library</span>
				{#if familyCount > 0}<span class="nav-count">{familyCount}</span>{/if}
			</button>
			<button
				use:registerItem={'nav:discover'}
				type="button"
				class:active={view === 'discover'}
				aria-current={view === 'discover' ? 'page' : undefined}
				title={collapsed ? 'Discover' : undefined}
				onpointerenter={() => (hoveredKey = 'nav:discover')}
				onfocus={() => (focusedKey = 'nav:discover')}
				onblur={() => (focusedKey = null)}
				onclick={() => onNavigate('discover')}
			>
				<Icon name="font" size={17} />
				<span class="nav-label">Discover</span>
			</button>
			<button
				use:registerItem={'nav:conflicts'}
				type="button"
				class:active={view === 'duplicates'}
				aria-current={view === 'duplicates' ? 'page' : undefined}
				title={collapsed ? 'Conflicts' : undefined}
				onpointerenter={() => (hoveredKey = 'nav:conflicts')}
				onfocus={() => (focusedKey = 'nav:conflicts')}
				onblur={() => (focusedKey = null)}
				onclick={() => onNavigate('duplicates')}
			>
				<Icon name="duplicates" size={17} />
				<span class="nav-label">Conflicts</span>
				{#if conflictCount > 0}<span class="nav-count warning">{conflictCount}</span>{/if}
			</button>
		</nav>

		{#if pinnedFamilies.length}
			<nav class="preview-navigation" aria-label="Saved font previews. Drag to reorder.">
				<p class="nav-group-label">Saved previews</p>
				{#each pinnedFamilies as family (family.id)}
					<div
						class:active={view === 'preview' && activeFamilyId === family.id}
						class:dragging={draggedFamilyId === family.id}
						class:drop-before={dropTarget?.familyId === family.id &&
							dropTarget.position === 'before'}
						class:drop-after={dropTarget?.familyId === family.id &&
							dropTarget.position === 'after'}
						class="preview-nav-item"
						draggable="true"
						role="group"
						aria-label={`${family.name} saved preview`}
						ondragstart={(event) => handlePreviewDragStart(event, family.id)}
						ondragover={(event) => handlePreviewDragOver(event, family.id)}
						ondrop={(event) => handlePreviewDrop(event, family.id)}
						ondragend={handlePreviewDragEnd}
					>
						<button
							use:registerItem={`preview:${family.id}`}
							type="button"
							class:active={view === 'preview' && activeFamilyId === family.id}
							class="preview-nav-open"
							aria-current={view === 'preview' && activeFamilyId === family.id
								? 'page'
								: undefined}
							title={family.name}
							onpointerenter={() => (hoveredKey = `preview:${family.id}`)}
							onfocus={() => (focusedKey = `preview:${family.id}`)}
							onblur={() => (focusedKey = null)}
							onclick={() => onOpenPreview(family.id)}
						>
							<Icon name="font" size={17} />
							<span class="nav-label">{family.name}</span>
						</button>
						<button
							type="button"
							class="preview-nav-close"
							aria-label={`Close saved preview for ${family.name}`}
							title="Close saved preview"
							onclick={() => onClosePreview(family.id)}
						>
							<Icon name="close" size={14} />
						</button>
					</div>
				{/each}
			</nav>
		{/if}

		<div class="catalogue-status">
			<div
				bind:this={footerRowElement}
				class="status-actions"
				role="group"
				aria-label="Workspace actions"
			>
				{#each footerActions as action (action.key)}
					<button
						use:registerItem={`action:${action.key}`}
						type="button"
						class="action-button"
						class:active={activeKey === `action:${action.key}`}
						class:expanded={expandedActionKey === action.key}
						disabled={action.disabled}
						aria-label={action.badge
							? `${action.label} (new release available)`
							: action.label}
						aria-current={activeKey === `action:${action.key}` ? 'page' : undefined}
						title={action.label}
						onpointerenter={() => (hoveredKey = `action:${action.key}`)}
						onfocus={() => (focusedKey = `action:${action.key}`)}
						onblur={() => (focusedKey = null)}
						onclick={action.onSelect}
					>
						<span class="action-icon">
							<Icon name={action.icon} size={16} />
							{#if action.badge}<span class="unseen-dot" aria-hidden="true"
								></span>{/if}
						</span>
						<span class="action-label">{action.label}</span>
					</button>
				{/each}
			</div>
		</div>
	</div>

	<div class="sidebar-edge-zone">
		<button
			type="button"
			class="sidebar-toggle"
			aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
			title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
			onclick={onToggle}
		>
			<span class:collapsed class="toggle-icon"><Icon name="chevron" size={14} /></span>
		</button>
	</div>
</aside>

<style>
	.app-navigation {
		position: sticky;
		top: 0;
		z-index: var(--z-sticky);
		height: var(--app-content-height, 100dvh);
		min-width: 0;
		align-self: start;
		overflow: visible;
		border-right: 1px solid var(--color-border);
		background: var(--color-panel);
	}

	.navigation-content {
		position: relative;
		display: flex;
		height: 100%;
		min-width: 0;
		flex-direction: column;
		overflow-x: hidden;
		overflow-y: auto;
		padding: var(--space-lg) var(--space-md) var(--space-md);
	}

	/* One highlight shared by every sidebar item; slides between nav, previews, and footer. */
	.sidebar-indicator {
		position: absolute;
		top: 0;
		left: 0;
		z-index: 0;
		border-radius: var(--radius-md);
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

	.sidebar-edge-zone {
		position: absolute;
		top: 0;
		right: -7px;
		z-index: 1;
		width: 14px;
		height: 100%;
	}

	.sidebar-toggle {
		position: absolute;
		top: 50%;
		left: 50%;
		display: grid;
		width: 26px;
		height: 38px;
		flex: none;
		place-items: center;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-muted);
		background: var(--color-panel);
		box-shadow: 0 4px 16px color-mix(in srgb, var(--color-bg) 72%, transparent);
		cursor: pointer;
		opacity: 0;
		pointer-events: none;
		transform: translate(-50%, -50%) scale(0.9);
		transition:
			opacity var(--motion-fast),
			color var(--motion-fast),
			background var(--motion-fast),
			border-color var(--motion-fast),
			transform var(--motion-fast);
	}

	.app-navigation:hover .sidebar-toggle,
	.app-navigation:focus-within .sidebar-toggle,
	.sidebar-toggle:focus-visible {
		opacity: 1;
		pointer-events: auto;
		transform: translate(-50%, -50%) scale(1);
	}

	.sidebar-toggle:hover {
		color: var(--color-text);
		border-color: var(--color-subtle);
		background: var(--color-control);
	}

	.sidebar-toggle:active {
		transform: translate(-50%, -50%) scale(0.94);
	}

	.toggle-icon {
		display: flex;
		transform: rotate(180deg);
		transition: transform var(--motion-standard);
	}

	.toggle-icon.collapsed {
		transform: rotate(0deg);
	}

	nav {
		position: relative;
		z-index: 1;
		display: grid;
		gap: 3px;
		margin-top: 12px;
	}

	.preview-navigation {
		margin-top: 16px;
	}

	.preview-navigation {
		gap: 0;
	}

	.nav-group-label {
		margin: 0 9px 6px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-weight: 650;
		letter-spacing: 0.045em;
		text-transform: uppercase;
	}

	nav button {
		display: grid;
		grid-template-columns: 18px minmax(0, 1fr) auto;
		min-height: 38px;
		align-items: center;
		gap: 9px;
		padding: 0 10px;
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		font-weight: 600;
		text-align: left;
		cursor: pointer;
		transition:
			color var(--motion-fast),
			background var(--motion-fast),
			transform var(--motion-fast);
	}

	nav button:hover {
		color: var(--color-text);
	}

	nav button:active {
		transform: translateY(1px);
	}

	nav button.active {
		color: var(--color-text);
	}

	.nav-label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.preview-nav-item {
		position: relative;
		min-width: 0;
		padding-block: 1.5px;
		user-select: none;
		transition: opacity var(--motion-fast);
	}

	.preview-nav-item.dragging {
		opacity: 0.46;
	}

	.preview-nav-item.drop-before::before,
	.preview-nav-item.drop-after::after {
		position: absolute;
		right: 8px;
		left: 8px;
		z-index: 2;
		height: 2px;
		border-radius: var(--radius-shell);
		background: var(--color-accent);
		content: '';
		pointer-events: none;
	}

	.preview-nav-item.drop-before::before {
		top: 0;
	}

	.preview-nav-item.drop-after::after {
		bottom: 0;
	}

	.preview-nav-open {
		width: 100%;
		padding-right: 40px;
		cursor: grab;
	}

	.preview-nav-open:active,
	.preview-nav-item.dragging .preview-nav-open {
		cursor: grabbing;
	}

	.preview-nav-item:hover .preview-nav-open:not(.active),
	.preview-nav-item:focus-within .preview-nav-open:not(.active) {
		color: var(--color-text);
	}

	nav .preview-nav-close {
		position: absolute;
		top: 50%;
		right: 6px;
		display: grid;
		grid-template-columns: 1fr;
		width: 26px;
		min-height: 26px;
		place-items: center;
		padding: 0;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		opacity: 0;
		pointer-events: none;
		transform: translateY(-50%);
		transition:
			color var(--motion-fast),
			background var(--motion-fast),
			opacity var(--motion-fast);
	}

	.preview-nav-item:hover .preview-nav-close,
	.preview-nav-item:focus-within .preview-nav-close,
	.preview-nav-item.active .preview-nav-close,
	.preview-nav-close:focus-visible {
		opacity: 1;
		pointer-events: auto;
	}

	nav .preview-nav-close:hover {
		color: var(--color-text);
		background: var(--color-raised);
	}

	nav .preview-nav-close:active {
		transform: translateY(-50%) scale(0.92);
	}

	.nav-count {
		min-width: 24px;
		padding: 2px 6px;
		border-radius: var(--radius-shell);
		color: var(--color-muted);
		background: var(--color-raised);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
		text-align: center;
	}

	.nav-count.warning {
		color: var(--color-warning);
	}

	.catalogue-status {
		position: relative;
		z-index: 1;
		margin-top: auto;
		padding: 8px 2px 2px;
		border-top: 1px solid var(--color-border);
	}

	.status-actions {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 4px;
	}

	.action-button {
		position: relative;
		z-index: 1;
		display: flex;
		height: 32px;
		align-items: center;
		padding: 0 8px;
		border: 0;
		border-radius: var(--radius-md);
		color: var(--color-muted);
		background: transparent;
		cursor: pointer;
		transition: color var(--motion-fast);
	}

	.action-button.expanded {
		color: var(--color-text);
	}

	.action-button:disabled {
		cursor: wait;
		opacity: 0.5;
	}

	.action-icon {
		position: relative;
		display: grid;
		width: 16px;
		height: 16px;
		flex: none;
		place-items: center;
	}

	.action-label {
		max-width: 0;
		margin-left: 0;
		overflow: hidden;
		font-size: var(--text-label);
		font-weight: 650;
		white-space: nowrap;
		opacity: 0;
		transition:
			max-width var(--motion-standard),
			margin-left var(--motion-standard),
			opacity var(--motion-fast);
	}

	.action-button.expanded .action-label {
		max-width: 92px;
		margin-left: 7px;
		opacity: 1;
	}

	.unseen-dot {
		position: absolute;
		top: -2px;
		right: -2px;
		width: 6px;
		height: 6px;
		border: 1.5px solid var(--color-panel);
		border-radius: 50%;
		background: var(--color-warning);
	}

	@media (prefers-reduced-motion: reduce) {
		.sidebar-indicator,
		.action-button,
		.action-label {
			transition:
				opacity var(--motion-fast),
				color var(--motion-fast);
		}
	}

	@media (min-width: 820px) {
		.navigation-content {
			transition: padding var(--motion-standard);
		}

		.collapsed .navigation-content {
			padding-inline: 8px;
		}

		.collapsed nav button {
			display: flex;
			justify-content: center;
			padding-inline: 0;
		}

		.collapsed .preview-nav-close {
			display: none;
		}

		.collapsed .nav-label,
		.collapsed .nav-count,
		.collapsed .nav-group-label,
		.collapsed .action-label {
			display: none;
		}

		.collapsed .catalogue-status {
			padding-inline: 0;
		}

		/* Narrow rail: icon-only actions stacked vertically; the pill slides up and down. */
		.collapsed .status-actions {
			flex-direction: column;
		}

		.collapsed .action-button {
			width: 34px;
			justify-content: center;
			padding: 0;
		}
	}

	@media (max-width: 819px) {
		.app-navigation {
			height: auto;
			align-self: auto;
			border-right: 0;
			border-bottom: 1px solid var(--color-border);
		}

		.navigation-content {
			display: flex;
			width: 100%;
			height: auto;
			flex-direction: row;
			overflow-x: auto;
			overflow-y: hidden;
			padding: 8px 12px;
		}

		.nav-group-label,
		.catalogue-status {
			display: none;
		}

		.sidebar-edge-zone {
			display: none;
		}

		nav {
			display: flex;
			justify-content: flex-start;
			gap: 4px;
			margin: 0;
			overflow-x: auto;
		}

		.preview-navigation {
			margin-left: 4px;
		}

		.preview-nav-item.drop-before::before,
		.preview-nav-item.drop-after::after {
			top: 6px;
			bottom: 6px;
			width: 2px;
			height: auto;
		}

		.preview-nav-item.drop-before::before {
			right: auto;
			left: -2px;
		}

		.preview-nav-item.drop-after::after {
			right: -2px;
			left: auto;
		}

		nav button {
			display: inline-flex;
			min-width: 44px;
			min-height: 40px;
			justify-content: center;
			padding: 0 10px;
		}

		.nav-count {
			display: none;
		}
	}

	@media (max-width: 540px) {
		nav button .nav-label {
			display: none;
		}
	}
</style>
