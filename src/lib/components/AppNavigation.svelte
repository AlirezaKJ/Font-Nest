<script lang="ts" module>
	export type AppView = 'library' | 'duplicates' | 'settings';
</script>

<script lang="ts">
	import logoMarkup from '../../../assets/branding/logo.svg?raw';
	import Icon from './Icon.svelte';

	const logoDataUrl = `data:image/svg+xml,${encodeURIComponent(logoMarkup)}`;

	let {
		view,
		familyCount,
		conflictCount,
		loading,
		mode,
		onNavigate,
		onRefresh
	}: {
		view: AppView;
		familyCount: number;
		conflictCount: number;
		loading: boolean;
		mode: 'native' | 'browser';
		onNavigate: (view: AppView) => void;
		onRefresh: () => void;
	} = $props();
</script>

<aside class="app-navigation" aria-label="Application navigation">
	<div class="brand-lockup">
		<img src={logoDataUrl} alt="" class="brand-mark" />
		<div class="brand-copy">
			<strong>FontNest</strong>
			<span>Working type archive</span>
		</div>
	</div>

	<nav aria-label="Primary">
		<p class="nav-group-label">Workspace</p>
		<button
			type="button"
			class:active={view === 'library'}
			aria-current={view === 'library' ? 'page' : undefined}
			onclick={() => onNavigate('library')}
		>
			<Icon name="library" size={17} />
			<span>Library</span>
			{#if familyCount > 0}<span class="nav-count">{familyCount}</span>{/if}
		</button>
		<button
			type="button"
			class:active={view === 'duplicates'}
			aria-current={view === 'duplicates' ? 'page' : undefined}
			onclick={() => onNavigate('duplicates')}
		>
			<Icon name="duplicates" size={17} />
			<span>Conflicts</span>
			{#if conflictCount > 0}<span class="nav-count warning">{conflictCount}</span>{/if}
		</button>

		<p class="nav-group-label settings-label">System</p>
		<button
			type="button"
			class:active={view === 'settings'}
			aria-current={view === 'settings' ? 'page' : undefined}
			onclick={() => onNavigate('settings')}
		>
			<Icon name="settings" size={17} />
			<span>Settings</span>
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
</aside>

<style>
	.app-navigation {
		position: sticky;
		top: 0;
		display: flex;
		height: var(--app-content-height, 100dvh);
		min-width: 0;
		align-self: start;
		flex-direction: column;
		overflow-y: auto;
		padding: var(--space-lg) var(--space-md) var(--space-md);
		border-right: 1px solid var(--color-border);
		background: var(--color-panel);
	}

	.brand-lockup {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 0 8px;
	}

	.brand-mark {
		width: 32px;
		height: 32px;
		object-fit: contain;
		filter: var(--logo-filter);
	}

	.brand-copy {
		display: grid;
		min-width: 0;
		gap: 2px;
	}

	.brand-copy strong {
		font-size: var(--text-title);
		line-height: 1.2;
		letter-spacing: -0.02em;
	}

	.brand-copy span {
		overflow: hidden;
		color: var(--color-muted);
		font-size: var(--text-micro);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	nav {
		display: grid;
		gap: 3px;
		margin-top: 28px;
	}

	.nav-group-label {
		margin: 0 9px 6px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-weight: 650;
		letter-spacing: 0.045em;
		text-transform: uppercase;
	}

	.settings-label {
		margin-top: 16px;
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

	@keyframes status-pulse {
		50% {
			opacity: 0.45;
		}
	}

	@media (max-width: 819px) {
		.app-navigation {
			z-index: var(--z-sticky);
			display: grid;
			height: auto;
			align-self: auto;
			grid-template-columns: auto minmax(0, 1fr);
			align-items: center;
			overflow: visible;
			padding: 8px 12px;
			border-right: 0;
			border-bottom: 1px solid var(--color-border);
		}

		.brand-copy,
		.nav-group-label,
		.catalogue-status {
			display: none;
		}

		.brand-lockup {
			padding: 0;
		}

		.brand-mark {
			width: 30px;
			height: 30px;
		}

		nav {
			display: flex;
			justify-content: flex-end;
			gap: 4px;
			margin: 0;
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
		nav button span:not(.nav-count) {
			display: none;
		}
	}
</style>
