<script lang="ts">
	import { onMount } from 'svelte';
	import { getCurrentWindow } from '@tauri-apps/api/window';

	import logoMarkup from '../../../assets/branding/logo.svg?raw';
	import Icon from './Icon.svelte';

	type AppView = 'library' | 'duplicates' | 'settings';

	const logoDataUrl = `data:image/svg+xml,${encodeURIComponent(logoMarkup)}`;

	let {
		search,
		loading,
		theme,
		onSearch,
		onNavigate,
		onToggleTheme,
		onRefresh,
		onPreview
	}: {
		search: string;
		loading: boolean;
		theme: 'system' | 'light' | 'dark';
		onSearch: (value: string) => void;
		onNavigate: (view: AppView) => void;
		onToggleTheme: () => void;
		onRefresh: () => void;
		onPreview: () => void;
	} = $props();

	let menuOpen = $state(false);
	let isDesktop = $state(false);
	let isMaximized = $state(false);
	let appWindow: ReturnType<typeof getCurrentWindow> | null = null;

	onMount(() => {
		isDesktop = '__TAURI_INTERNALS__' in window;
		if (!isDesktop) return;

		appWindow = getCurrentWindow();
		let disposed = false;
		let unlistenResize: (() => void) | undefined;

		void syncMaximized();
		void appWindow
			.onResized(() => void syncMaximized())
			.then((unlisten) => {
				if (disposed) unlisten();
				else unlistenResize = unlisten;
			});

		return () => {
			disposed = true;
			unlistenResize?.();
		};
	});

	function runMenuAction(action: () => void) {
		menuOpen = false;
		action();
	}

	async function syncMaximized() {
		if (!appWindow) return;
		try {
			isMaximized = await appWindow.isMaximized();
		} catch (error) {
			console.error('FontNest could not read the window state.', error);
		}
	}

	async function minimizeWindow() {
		if (!appWindow) return;
		try {
			await appWindow.minimize();
		} catch (error) {
			console.error('FontNest could not minimize the window.', error);
		}
	}

	async function toggleMaximizeWindow() {
		if (!appWindow) return;
		try {
			await appWindow.toggleMaximize();
			await syncMaximized();
		} catch (error) {
			console.error('FontNest could not resize the window.', error);
		}
	}

	async function closeWindow() {
		if (!appWindow) return;
		try {
			await appWindow.close();
		} catch (error) {
			console.error('FontNest could not close the window.', error);
		}
	}
</script>

<header class="app-titlebar" data-tauri-drag-region>
	<div class="titlebar-left" data-tauri-drag-region>
		<details class="application-menu" bind:open={menuOpen}>
			<summary aria-label="Open FontNest menu" title="FontNest menu">
				<Icon name="more" size={19} />
			</summary>
			<div class="menu-panel" aria-label="FontNest menu">
				<p>Navigate</p>
				<button type="button" onclick={() => runMenuAction(() => onNavigate('library'))}>
					<Icon name="library" size={16} /> Library
				</button>
				<button type="button" onclick={() => runMenuAction(() => onNavigate('duplicates'))}>
					<Icon name="duplicates" size={16} /> Conflicts
				</button>
				<button type="button" onclick={() => runMenuAction(() => onNavigate('settings'))}>
					<Icon name="settings" size={16} /> Settings
				</button>
				<div class="menu-divider"></div>
				<p>Font tools</p>
				<button type="button" onclick={() => runMenuAction(onPreview)}>
					<Icon name="upload" size={16} /> Preview a font
				</button>
				<button type="button" disabled={loading} onclick={() => runMenuAction(onRefresh)}>
					<Icon name="refresh" size={16} />
					{loading ? 'Scanning fonts…' : 'Scan fonts again'}
				</button>
			</div>
		</details>

		<div class="titlebar-brand" data-tauri-drag-region>
			<img src={logoDataUrl} alt="" data-tauri-drag-region />
			<span data-tauri-drag-region>FontNest</span>
		</div>
	</div>

	<label class="titlebar-search">
		<span class="sr-only">Search font families</span>
		<Icon name="search" size={18} />
		<input
			data-font-search
			type="search"
			value={search}
			placeholder="Search your font library"
			autocomplete="off"
			oninput={(event) => onSearch(event.currentTarget.value)}
		/>
		{#if search}
			<button type="button" aria-label="Clear font search" onclick={() => onSearch('')}>
				<Icon name="close" size={16} />
			</button>
		{:else}
			<kbd>/</kbd>
		{/if}
	</label>

	<div class="titlebar-actions" data-tauri-drag-region>
		<button
			type="button"
			class:scanning={loading}
			class="titlebar-tool catalogue-tool"
			disabled={loading}
			onclick={onRefresh}
			aria-label={loading ? 'Scanning installed fonts' : 'Scan fonts again'}
			title={loading ? 'Scanning installed fonts' : 'Scan fonts again'}
		>
			<span class="catalogue-dot"></span>
			<span>{loading ? 'Scanning' : 'Catalogue ready'}</span>
		</button>
		<button
			type="button"
			class="titlebar-tool"
			onclick={onToggleTheme}
			aria-label="Toggle light and dark theme"
			title="Toggle light and dark theme"
		>
			<Icon name={theme === 'dark' ? 'sun' : 'moon'} size={17} />
		</button>

		<div class="window-controls" aria-label="Window controls">
			<button
				type="button"
				disabled={!isDesktop}
				onclick={() => void minimizeWindow()}
				aria-label="Minimize window"
				title="Minimize"
			>
				<Icon name="minimize" size={15} />
			</button>
			<button
				type="button"
				disabled={!isDesktop}
				onclick={() => void toggleMaximizeWindow()}
				aria-label={isMaximized ? 'Restore window' : 'Maximize window'}
				title={isMaximized ? 'Restore' : 'Maximize'}
			>
				<Icon name={isMaximized ? 'restore' : 'maximize'} size={14} />
			</button>
			<button
				type="button"
				class="close-control"
				disabled={!isDesktop}
				onclick={() => void closeWindow()}
				aria-label="Close window"
				title="Close"
			>
				<Icon name="close" size={16} />
			</button>
		</div>
	</div>
</header>

<style>
	.app-titlebar {
		position: relative;
		z-index: var(--z-titlebar);
		display: grid;
		height: var(--titlebar-height, 48px);
		grid-template-columns: minmax(160px, 1fr) minmax(320px, 520px) minmax(218px, 1fr);
		align-items: center;
		border-bottom: 1px solid #252620;
		color: #f1f0ea;
		background: #080907;
		font-size: var(--text-label);
		user-select: none;
	}

	.titlebar-left,
	.titlebar-actions,
	.titlebar-brand,
	.titlebar-tool,
	.window-controls {
		display: flex;
		align-items: center;
	}

	.titlebar-left {
		min-width: 0;
		align-self: stretch;
		gap: 6px;
		padding-left: 8px;
	}

	.titlebar-brand {
		min-width: 0;
		gap: 8px;
		color: #d7d6cf;
		font-weight: 600;
		letter-spacing: -0.01em;
	}

	.titlebar-brand img {
		width: 20px;
		height: 20px;
		object-fit: contain;
		filter: invert(1);
	}

	.application-menu {
		position: relative;
		flex: none;
	}

	.application-menu summary,
	.titlebar-tool,
	.titlebar-search button,
	.window-controls button {
		border: 0;
		color: #c8c7c0;
		background: transparent;
		cursor: pointer;
	}

	.application-menu summary,
	.titlebar-tool,
	.titlebar-search button {
		display: grid;
		width: 36px;
		height: 36px;
		place-items: center;
		border-radius: var(--radius-md);
		transition:
			color var(--motion-fast),
			background var(--motion-fast),
			transform var(--motion-fast);
	}

	.application-menu summary {
		list-style: none;
	}

	.application-menu summary::-webkit-details-marker {
		display: none;
	}

	.application-menu summary:hover,
	.titlebar-tool:hover:not(:disabled),
	.titlebar-search button:hover {
		color: #f1f0ea;
		background: #252620;
	}

	.application-menu summary:active,
	.titlebar-tool:active:not(:disabled),
	.titlebar-search button:active {
		transform: translateY(1px);
	}

	.menu-panel {
		position: absolute;
		z-index: var(--z-dropdown);
		top: 41px;
		left: 0;
		display: grid;
		width: 228px;
		gap: 2px;
		padding: 8px;
		border: 1px solid #34342f;
		border-radius: var(--radius-lg);
		color: #f1f0ea;
		background: #1d1e1a;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.38);
	}

	.menu-panel p {
		margin: 4px 8px 3px;
		color: #8c8b82;
		font-size: var(--text-micro);
		font-weight: 650;
	}

	.menu-panel button {
		display: grid;
		min-height: 34px;
		grid-template-columns: 18px minmax(0, 1fr);
		align-items: center;
		gap: 9px;
		padding: 0 9px;
		border: 0;
		border-radius: var(--radius-sm);
		color: #d7d6cf;
		background: transparent;
		font-size: var(--text-label);
		font-weight: 600;
		text-align: left;
		cursor: pointer;
	}

	.menu-panel button:hover:not(:disabled) {
		color: #f1f0ea;
		background: #2c2d27;
	}

	.menu-panel button:disabled {
		color: #8c8b82;
		cursor: wait;
	}

	.menu-divider {
		height: 1px;
		margin: 5px 4px;
		background: #34342f;
	}

	.titlebar-search {
		position: relative;
		display: flex;
		min-width: 0;
		align-items: center;
		color: #aaa9a0;
	}

	.titlebar-search > :global(svg) {
		position: absolute;
		left: 12px;
		pointer-events: none;
	}

	.titlebar-search input {
		width: 100%;
		height: 36px;
		padding: 0 42px 0 39px;
		border: 1px solid #34342f;
		border-radius: var(--radius-shell);
		outline: none;
		color: #f1f0ea;
		background: #20211e;
		font-size: var(--text-body-sm);
		font-weight: 550;
		user-select: text;
		transition:
			border-color var(--motion-fast),
			background var(--motion-fast);
	}

	.titlebar-search input:hover {
		background: #242520;
	}

	.titlebar-search input:focus-visible {
		border-color: #c6b98f;
		outline: 2px solid #c6b98f;
		outline-offset: 1px;
	}

	.titlebar-search input::placeholder {
		color: #aaa9a0;
		opacity: 1;
	}

	.titlebar-search input::-webkit-search-cancel-button {
		display: none;
	}

	.titlebar-search button,
	.titlebar-search kbd {
		position: absolute;
		right: 5px;
	}

	.titlebar-search button {
		width: 30px;
		height: 30px;
		border-radius: 50%;
	}

	.titlebar-search kbd {
		display: grid;
		width: 24px;
		height: 22px;
		place-items: center;
		border: 1px solid #45463f;
		border-radius: var(--radius-xs);
		color: #aaa9a0;
		background: #292a25;
		font: inherit;
		font-size: var(--text-micro);
	}

	.titlebar-actions {
		min-width: 0;
		align-self: stretch;
		justify-content: flex-end;
		gap: 2px;
	}

	.titlebar-tool {
		flex: none;
	}

	.catalogue-tool {
		width: auto;
		grid-auto-flow: column;
		gap: 7px;
		padding: 0 9px;
		color: #aaa9a0;
		font-size: var(--text-micro);
		font-weight: 600;
	}

	.catalogue-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: #71b184;
	}

	.catalogue-tool.scanning .catalogue-dot {
		background: #e1af59;
		animation: catalogue-pulse 1.2s ease-in-out infinite;
	}

	.window-controls {
		align-self: stretch;
		margin-left: 5px;
	}

	.window-controls button {
		display: grid;
		width: 46px;
		height: 100%;
		place-items: center;
		border-radius: 0;
		transition:
			color var(--motion-fast),
			background var(--motion-fast);
	}

	.window-controls button:hover:not(:disabled) {
		color: #ffffff;
		background: #292a26;
	}

	.window-controls .close-control:hover:not(:disabled) {
		background: #c42b1c;
	}

	.window-controls button:active:not(:disabled) {
		background: #34352f;
	}

	.window-controls .close-control:active:not(:disabled) {
		background: #9f2117;
	}

	.window-controls button:disabled {
		cursor: default;
		opacity: 0.42;
	}

	.app-titlebar :where(button, input, summary):focus-visible {
		outline-color: #c6b98f;
	}

	@keyframes catalogue-pulse {
		50% {
			opacity: 0.4;
		}
	}

	@media (max-width: 760px) {
		.app-titlebar {
			grid-template-columns: 52px minmax(180px, 1fr) 138px;
		}

		.titlebar-brand,
		.titlebar-tool {
			display: none;
		}

		.window-controls {
			margin-left: 0;
		}
	}
</style>
