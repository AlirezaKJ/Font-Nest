<script lang="ts" module>
	export type AppView = 'library' | 'discover' | 'duplicates' | 'preview' | 'settings';
	export type PinnedFamily = { id: string; name: string };
</script>

<script lang="ts">
	import { getDirectionalReorderPosition, type ReorderPosition } from '$lib/reorder';

	import Icon from './Icon.svelte';

	let {
		view,
		familyCount,
		conflictCount,
		loading,
		mode,
		collapsed,
		pinnedFamilies,
		activeFamilyId,
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
		mode: 'native' | 'browser';
		collapsed: boolean;
		pinnedFamilies: PinnedFamily[];
		activeFamilyId: string | null;
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
</script>

<aside class:collapsed class="app-navigation" aria-label="Application navigation">
	<div class="navigation-content">
		<nav aria-label="Primary">
			<p class="nav-group-label">Workspace</p>
			<button
				type="button"
				class:active={view === 'library'}
				aria-current={view === 'library' ? 'page' : undefined}
				title={collapsed ? 'Library' : undefined}
				onclick={() => onNavigate('library')}
			>
				<Icon name="library" size={17} />
				<span class="nav-label">Library</span>
				{#if familyCount > 0}<span class="nav-count">{familyCount}</span>{/if}
			</button>
			<button
				type="button"
				class:active={view === 'discover'}
				aria-current={view === 'discover' ? 'page' : undefined}
				title={collapsed ? 'Discover' : undefined}
				onclick={() => onNavigate('discover')}
			>
				<Icon name="font" size={17} />
				<span class="nav-label">Discover</span>
			</button>
			<button
				type="button"
				class:active={view === 'duplicates'}
				aria-current={view === 'duplicates' ? 'page' : undefined}
				title={collapsed ? 'Conflicts' : undefined}
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
							type="button"
							class:active={view === 'preview' && activeFamilyId === family.id}
							class="preview-nav-open"
							aria-current={view === 'preview' && activeFamilyId === family.id
								? 'page'
								: undefined}
							title={collapsed ? family.name : undefined}
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

		<nav class="system-navigation" aria-label="System navigation">
			<p class="nav-group-label">System</p>
			<button
				type="button"
				class:active={view === 'settings'}
				aria-current={view === 'settings' ? 'page' : undefined}
				title={collapsed ? 'Settings' : undefined}
				onclick={() => onNavigate('settings')}
			>
				<Icon name="settings" size={17} />
				<span class="nav-label">Settings</span>
			</button>
		</nav>

		<div class="catalogue-status">
			<div class="status-line">
				<span class:scanning={loading} class="status-indicator"></span>
				<strong
					>{loading
						? 'Scanning catalogue'
						: mode === 'native'
							? 'Catalogue ready'
							: 'Browser preview'}</strong
				>
				<button
					type="button"
					class="icon-button"
					disabled={loading}
					aria-label="Scan fonts again"
					title="Scan fonts again"
					onclick={onRefresh}
				>
					<Icon name="refresh" size={15} />
				</button>
			</div>
			<p>
				{mode === 'native'
					? `${familyCount.toLocaleString()} installed families`
					: 'Sample data for UI development'}
			</p>
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
		display: flex;
		height: 100%;
		min-width: 0;
		flex-direction: column;
		overflow-x: hidden;
		overflow-y: auto;
		padding: var(--space-lg) var(--space-md) var(--space-md);
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
		display: grid;
		gap: 3px;
		margin-top: 12px;
	}

	.preview-navigation,
	.system-navigation {
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
		background: var(--color-hover);
	}

	nav button:active {
		transform: translateY(1px);
	}

	nav button.active {
		color: var(--color-text);
		background: var(--color-selected);
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
		background: var(--color-hover);
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
		margin-top: auto;
		padding: 12px 8px 2px;
		border-top: 1px solid var(--color-border);
	}

	.status-line {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 7px;
		font-size: var(--text-label);
	}

	.status-line strong {
		overflow: hidden;
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.status-indicator {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--color-success);
	}

	.status-indicator.scanning {
		background: var(--color-warning);
		animation: status-pulse 1.2s ease-in-out infinite;
	}

	.catalogue-status p {
		margin: 5px 0 0 14px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		line-height: 1.4;
	}

	.icon-button {
		display: grid;
		width: 30px;
		height: 30px;
		place-items: center;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		cursor: pointer;
	}

	.icon-button:hover:not(:disabled) {
		color: var(--color-text);
		background: var(--color-selected);
	}

	.icon-button:disabled {
		cursor: wait;
		opacity: 0.5;
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
		.collapsed .status-line strong,
		.collapsed .catalogue-status p {
			display: none;
		}

		.collapsed .catalogue-status {
			padding-inline: 0;
		}

		.collapsed .status-line {
			display: flex;
			flex-direction: column;
			gap: 5px;
		}
	}

	@keyframes status-pulse {
		50% {
			opacity: 0.45;
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

		.system-navigation {
			margin-left: 4px;
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
