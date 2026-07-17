<script lang="ts">
	import { onMount } from 'svelte';

	import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
	import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
	import AppNavigation, { type AppView } from '$lib/components/AppNavigation.svelte';
	import ConflictsView from '$lib/components/ConflictsView.svelte';
	import FontInspector from '$lib/components/FontInspector.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SettingsView, {
		type DensityPreference,
		type ThemePreference
	} from '$lib/components/SettingsView.svelte';
	import { createBrowserCatalogue } from '$lib/catalogue/browser-catalogue';
	import { scanInstalledFonts } from '$lib/tauri/commands';

	const PAGE_SIZE = 120;
	const SKELETON_ROWS = [0, 1, 2, 3, 4, 5, 6];
	const DEFAULT_PREVIEW = 'Hamburgefontsiv 0123 — Pack my box with five dozen liquor jugs.';
	const PREFERENCES_KEY = 'fontnest.preferences.v1';

	type CatalogueMode = 'native' | 'browser';
	type Toast = { message: string; tone: 'success' | 'error' };

	let view = $state<AppView>('library');
	let catalogue = $state<FontCatalogue | null>(null);
	let catalogueMode = $state<CatalogueMode>('browser');
	let loading = $state(true);
	let errorMessage = $state('');
	let selectedFamilyId = $state<string | null>(null);
	let search = $state('');
	let sourceFilter = $state('All');
	let formatFilter = $state('All');
	let monospacedOnly = $state(false);
	let conflictsOnly = $state(false);
	let displayLimit = $state(PAGE_SIZE);
	let previewText = $state(DEFAULT_PREVIEW);
	let previewSize = $state(64);
	let previewWeight = $state(400);
	let theme = $state<ThemePreference>('dark');
	let density = $state<DensityPreference>('comfortable');
	let toast = $state<Toast | null>(null);
	let searchInput = $state<HTMLInputElement>();
	let previewFileInput = $state<HTMLInputElement>();
	let toastTimer: ReturnType<typeof setTimeout> | undefined;

	let sourceOptions = $derived.by(() => {
		const values = new Set(catalogue?.families.flatMap((family) => family.sources) ?? []);
		return ['All', ...Array.from(values).sort()];
	});

	let formatOptions = $derived.by(() => {
		const values = new Set(catalogue?.families.flatMap((family) => family.formats) ?? []);
		return ['All', ...Array.from(values).sort()];
	});

	let activeFilterCount = $derived(
		Number(sourceFilter !== 'All') +
			Number(formatFilter !== 'All') +
			Number(monospacedOnly) +
			Number(conflictsOnly)
	);

	let filteredFamilies = $derived.by(() => {
		const terms = search
			.toLocaleLowerCase()
			.split(/\s+/)
			.map((term) => term.trim())
			.filter(Boolean);

		return (catalogue?.families ?? []).filter((family) => {
			const searchable = [family.name, ...family.styles, ...family.sources, ...family.formats]
				.join(' ')
				.toLocaleLowerCase();
			return (
				terms.every((term) => searchable.includes(term)) &&
				(sourceFilter === 'All' || family.sources.includes(sourceFilter)) &&
				(formatFilter === 'All' || family.formats.includes(formatFilter)) &&
				(!monospacedOnly || family.monospaced) &&
				(!conflictsOnly || family.hasConflict)
			);
		});
	});

	let renderedFamilies = $derived(filteredFamilies.slice(0, displayLimit));
	let selectedFamily = $derived.by(() => {
		const selected =
			catalogue?.families.find((family) => family.id === selectedFamilyId) ?? null;
		if (
			filteredFamilies.length &&
			!filteredFamilies.some((family) => family.id === selected?.id)
		) {
			return filteredFamilies[0];
		}
		return selected ?? filteredFamilies[0] ?? null;
	});
	let conflictFamilies = $derived(
		catalogue?.families.filter((family) => family.hasConflict) ?? []
	);

	onMount(() => {
		loadPreferences();
		applyTheme();

		const colorScheme = window.matchMedia('(prefers-color-scheme: dark)');
		const handleColorScheme = () => {
			if (theme === 'system') applyTheme();
		};
		const handleKeydown = (event: KeyboardEvent) => {
			const target = event.target as HTMLElement | null;
			const isEditing =
				target?.matches('input, textarea, select, [contenteditable="true"]') ?? false;

			if (event.key === '/' && !isEditing) {
				event.preventDefault();
				view = 'library';
				requestAnimationFrame(() => searchInput?.focus());
			} else if (event.key === 'Escape' && search) {
				search = '';
				displayLimit = PAGE_SIZE;
				searchInput?.focus();
			}
		};

		colorScheme.addEventListener('change', handleColorScheme);
		window.addEventListener('keydown', handleKeydown);
		void refreshCatalogue();

		return () => {
			colorScheme.removeEventListener('change', handleColorScheme);
			window.removeEventListener('keydown', handleKeydown);
			if (toastTimer) clearTimeout(toastTimer);
		};
	});

	function loadPreferences() {
		try {
			const saved = JSON.parse(localStorage.getItem(PREFERENCES_KEY) ?? '{}') as {
				theme?: ThemePreference;
				density?: DensityPreference;
				previewText?: string;
			};
			if (saved.theme && ['system', 'light', 'dark'].includes(saved.theme))
				theme = saved.theme;
			if (saved.density && ['comfortable', 'compact'].includes(saved.density)) {
				density = saved.density;
			}
			if (saved.previewText?.trim()) previewText = saved.previewText;
		} catch {
			localStorage.removeItem(PREFERENCES_KEY);
		}
	}

	function savePreferences() {
		localStorage.setItem(PREFERENCES_KEY, JSON.stringify({ theme, density, previewText }));
	}

	function applyTheme() {
		const resolved =
			theme === 'system'
				? window.matchMedia('(prefers-color-scheme: dark)').matches
					? 'dark'
					: 'light'
				: theme;
		document.documentElement.dataset.theme = resolved;
		document.documentElement.style.colorScheme = resolved;
	}

	function setTheme(value: ThemePreference) {
		theme = value;
		applyTheme();
		savePreferences();
	}

	function toggleTheme() {
		const resolved = document.documentElement.dataset.theme;
		setTheme(resolved === 'dark' ? 'light' : 'dark');
	}

	function setDensity(value: DensityPreference) {
		density = value;
		savePreferences();
	}

	function setPreviewText(value: string) {
		previewText = value;
		savePreferences();
	}

	async function refreshCatalogue() {
		loading = true;
		errorMessage = '';
		const isNative = '__TAURI_INTERNALS__' in window;
		catalogueMode = isNative ? 'native' : 'browser';

		if (!isNative) {
			catalogue = createBrowserCatalogue();
			selectedFamilyId = catalogue.families[0]?.id ?? null;
			previewWeight = nearestWeight(catalogue.families[0]?.weights ?? [400], 400);
			loading = false;
			return;
		}

		try {
			catalogue = await scanInstalledFonts();
			selectedFamilyId = catalogue.families[0]?.id ?? null;
			previewWeight = nearestWeight(catalogue.families[0]?.weights ?? [400], 400);
		} catch (error) {
			catalogue = null;
			errorMessage = commandErrorMessage(error);
		} finally {
			loading = false;
		}
	}

	function commandErrorMessage(error: unknown): string {
		if (typeof error === 'object' && error && 'message' in error) {
			return String(error.message);
		}
		return 'FontNest could not read the installed font catalogue. Try scanning again.';
	}

	function nearestWeight(weights: number[], target: number): number {
		return weights.reduce(
			(closest, weight) =>
				Math.abs(weight - target) < Math.abs(closest - target) ? weight : closest,
			weights[0] ?? 400
		);
	}

	function selectFamily(familyId: string) {
		selectedFamilyId = familyId;
		const family = catalogue?.families.find((candidate) => candidate.id === familyId);
		if (family) previewWeight = nearestWeight(family.weights, previewWeight);
	}

	function inspectConflict(familyId: string) {
		selectFamily(familyId);
		view = 'library';
	}

	function handleRowKeydown(event: KeyboardEvent, index: number) {
		if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
		event.preventDefault();

		let nextIndex = index;
		if (event.key === 'ArrowDown') nextIndex = Math.min(index + 1, renderedFamilies.length - 1);
		if (event.key === 'ArrowUp') nextIndex = Math.max(index - 1, 0);
		if (event.key === 'Home') nextIndex = 0;
		if (event.key === 'End') nextIndex = renderedFamilies.length - 1;

		const next = renderedFamilies[nextIndex];
		if (!next) return;
		selectFamily(next.id);
		document.querySelectorAll<HTMLButtonElement>('.font-row')[nextIndex]?.focus();
	}

	function updateSearch(value: string) {
		search = value;
		displayLimit = PAGE_SIZE;
	}

	function clearFilters() {
		sourceFilter = 'All';
		formatFilter = 'All';
		monospacedOnly = false;
		conflictsOnly = false;
		displayLimit = PAGE_SIZE;
	}

	function safeFontStack(name: string): string {
		return `"${name.replace(/["\\;\n\r]/g, '')}", system-ui, sans-serif`;
	}

	function familyPreviewStyle(family: FontFamilySummary): string {
		return `font-family: ${safeFontStack(family.name)}; font-weight: ${nearestWeight(family.weights, 400)};`;
	}

	function familyMeta(family: FontFamilySummary): string {
		return [
			family.sources.join(' · '),
			family.formats.join(' · '),
			family.monospaced ? 'Monospaced' : ''
		]
			.filter(Boolean)
			.join(' · ');
	}

	function openPreviewFilePicker() {
		previewFileInput?.click();
	}

	async function previewFontFile(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;

		try {
			if (file.size > 30 * 1024 * 1024) {
				throw new Error('Choose a font file smaller than 30 MB for preview.');
			}

			const extension = file.name.split('.').pop()?.toLocaleLowerCase() ?? '';
			const format =
				extension === 'otf'
					? 'OpenType'
					: extension === 'ttf'
						? 'TrueType'
						: extension === 'woff2'
							? 'WOFF2 preview'
							: 'WOFF preview';
			const displayName =
				file.name
					.replace(/\.(otf|ttf|woff2?)$/i, '')
					.replace(/[-_]+/g, ' ')
					.replace(/["\\;\n\r]/g, '')
					.trim() || 'Preview font';
			const runtimeFamily = `FontNestPreview${Date.now()}`;
			const font = new FontFace(displayName, await file.arrayBuffer());
			await font.load();
			document.fonts.add(font);

			const id = `preview:${runtimeFamily.toLocaleLowerCase()}`;
			const family: FontFamilySummary = {
				id,
				name: displayName,
				faceCount: 1,
				fileCount: 1,
				styles: ['Preview'],
				weights: [400],
				formats: [format],
				sources: ['Preview only'],
				monospaced: false,
				hasConflict: false,
				faces: [
					{
						id: `${id}:0`,
						postScriptName: displayName.replaceAll(' ', ''),
						styleName: 'Preview',
						style: 'normal',
						weight: 400,
						format,
						source: 'Preview only',
						fileName: file.name,
						faceIndex: 0,
						monospaced: false
					}
				]
			};

			if (!catalogue) catalogue = createBrowserCatalogue();
			catalogue = {
				...catalogue,
				families: [family, ...catalogue.families],
				familyCount: catalogue.familyCount + 1,
				faceCount: catalogue.faceCount + 1
			};
			selectedFamilyId = family.id;
			view = 'library';
			showToast(`${file.name} is ready to preview. It was not installed.`, 'success');
		} catch (error) {
			showToast(
				error instanceof Error ? error.message : 'FontNest could not preview that file.',
				'error'
			);
		} finally {
			input.value = '';
		}
	}

	function showToast(message: string, tone: Toast['tone']) {
		if (toastTimer) clearTimeout(toastTimer);
		toast = { message, tone };
		toastTimer = setTimeout(() => {
			toast = null;
		}, 5000);
	}
</script>

<svelte:head>
	<title>FontNest — Working type archive</title>
	<meta
		name="description"
		content="Browse, preview, and inspect the fonts installed on your computer."
	/>
</svelte:head>

<a class="skip-link" href="#main-content">Skip to font catalogue</a>
<input
	bind:this={previewFileInput}
	class="visually-hidden-file"
	type="file"
	accept=".otf,.ttf,.woff,.woff2,font/otf,font/ttf,font/woff,font/woff2"
	aria-hidden="true"
	tabindex="-1"
	onchange={previewFontFile}
/>

<div class:compact={density === 'compact'} class="app-shell">
	<AppNavigation
		{view}
		familyCount={catalogue?.familyCount ?? 0}
		conflictCount={catalogue?.conflictCount ?? 0}
		{loading}
		mode={catalogueMode}
		onNavigate={(nextView) => (view = nextView)}
		onRefresh={() => void refreshCatalogue()}
	/>

	<main id="main-content">
		{#if view === 'library'}
			<section class="library-workspace" aria-labelledby="library-title">
				<div class="library-pane">
					<header class="library-header">
						<div class="heading-row">
							<div>
								<p class="section-label">Font library</p>
								<h1 id="library-title">Your fonts</h1>
								<p class="catalogue-summary">
									{#if catalogue}
										{catalogue.familyCount.toLocaleString()} families · {catalogue.faceCount.toLocaleString()}
										faces
										{#if catalogueMode === 'native'}
											· scanned in {catalogue.scanDurationMs.toLocaleString()} ms
										{/if}
									{:else if loading}
										Reading the installed font catalogue…
									{:else}
										Catalogue unavailable
									{/if}
								</p>
							</div>
							<div class="header-actions">
								<button
									type="button"
									class="icon-action"
									onclick={toggleTheme}
									aria-label="Toggle light and dark theme"
									title="Toggle light and dark theme"
								>
									<Icon name={theme === 'dark' ? 'sun' : 'moon'} size={17} />
								</button>
								<button
									type="button"
									class="primary-action"
									onclick={openPreviewFilePicker}
								>
									<Icon name="upload" size={16} /> <span>Preview a font</span>
								</button>
							</div>
						</div>

						<div class="toolbar">
							<label class="search-field">
								<span class="sr-only">Search font families</span>
								<Icon name="search" size={17} />
								<input
									bind:this={searchInput}
									type="search"
									value={search}
									placeholder="Search families, styles, or formats"
									oninput={(event) => updateSearch(event.currentTarget.value)}
								/>
								<kbd>/</kbd>
							</label>

							<details class="filter-menu">
								<summary>
									<Icon name="filter" size={16} />
									Filters{#if activeFilterCount}
										· {activeFilterCount}{/if}
								</summary>
								<div class="filter-panel">
									<div class="filter-panel-head">
										<strong>Filter catalogue</strong>
										<button
											type="button"
											disabled={!activeFilterCount}
											onclick={clearFilters}
										>
											Clear
										</button>
									</div>
									<label>
										<span>Source</span>
										<select
											value={sourceFilter}
											onchange={(event) => {
												sourceFilter = event.currentTarget.value;
												displayLimit = PAGE_SIZE;
											}}
										>
											{#each sourceOptions as option (option)}<option
													value={option}>{option}</option
												>{/each}
										</select>
									</label>
									<label>
										<span>Format</span>
										<select
											value={formatFilter}
											onchange={(event) => {
												formatFilter = event.currentTarget.value;
												displayLimit = PAGE_SIZE;
											}}
										>
											{#each formatOptions as option (option)}<option
													value={option}>{option}</option
												>{/each}
										</select>
									</label>
									<label class="check-filter">
										<input type="checkbox" bind:checked={monospacedOnly} />
										<span>Monospaced families only</span>
									</label>
									<label class="check-filter">
										<input type="checkbox" bind:checked={conflictsOnly} />
										<span>Potential conflicts only</span>
									</label>
								</div>
							</details>
						</div>
					</header>

					<div class="table-head" aria-hidden="true">
						<span>Family</span><span>Preview</span><span>Styles</span>
					</div>

					<div class="font-list" role="listbox" aria-label="Font families">
						{#if loading && !catalogue}
							{#each SKELETON_ROWS as index (index)}
								<div class="font-row skeleton-row" aria-hidden="true">
									<span
										class="skeleton family-skeleton"
										style={`--skeleton-index: ${index}`}
									></span>
									<span
										class="skeleton preview-skeleton"
										style={`--skeleton-index: ${index}`}
									></span>
									<span
										class="skeleton count-skeleton"
										style={`--skeleton-index: ${index}`}
									></span>
								</div>
							{/each}
						{:else if errorMessage}
							<div class="state-view" role="alert">
								<div class="state-icon error"><Icon name="alert" size={22} /></div>
								<h2>Catalogue scan did not finish</h2>
								<p>{errorMessage}</p>
								<button type="button" onclick={() => void refreshCatalogue()}
									>Scan again</button
								>
							</div>
						{:else if !catalogue?.familyCount}
							<div class="state-view">
								<div class="state-icon"><Icon name="font" size={23} /></div>
								<h2>No installed fonts found</h2>
								<p>
									Scan again, or open a font file to preview it without installing
									anything.
								</p>
								<button type="button" onclick={openPreviewFilePicker}
									>Preview a font file</button
								>
							</div>
						{:else if !renderedFamilies.length}
							<div class="state-view">
								<div class="state-icon"><Icon name="search" size={22} /></div>
								<h2>No families match</h2>
								<p>Try a shorter search, or clear the active catalogue filters.</p>
								<button
									type="button"
									onclick={() => {
										search = '';
										clearFilters();
									}}
								>
									Clear search and filters
								</button>
							</div>
						{:else}
							{#each renderedFamilies as family, index (family.id)}
								<button
									type="button"
									class:selected={selectedFamily?.id === family.id}
									class="font-row"
									role="option"
									aria-selected={selectedFamily?.id === family.id}
									tabindex={selectedFamily?.id === family.id ? 0 : -1}
									onclick={() => selectFamily(family.id)}
									onkeydown={(event) => handleRowKeydown(event, index)}
								>
									<span class="family-cell">
										<strong>{family.name}</strong>
										<small>{familyMeta(family)}</small>
									</span>
									<span class="preview-cell" style={familyPreviewStyle(family)}>
										{previewText || DEFAULT_PREVIEW}
									</span>
									<span class="style-cell">
										{family.faceCount}
										{family.faceCount === 1 ? 'style' : 'styles'}
										{#if family.hasConflict}<small
												><Icon name="alert" size={12} /> Conflict</small
											>{/if}
									</span>
								</button>
							{/each}
							{#if renderedFamilies.length < filteredFamilies.length}
								<div class="load-more-row">
									<button
										type="button"
										onclick={() => (displayLimit += PAGE_SIZE)}
									>
										Show {Math.min(
											PAGE_SIZE,
											filteredFamilies.length - renderedFamilies.length
										)} more
									</button>
								</div>
							{/if}
						{/if}
					</div>
				</div>

				<FontInspector
					family={selectedFamily}
					{previewText}
					{previewSize}
					{previewWeight}
					onPreviewText={setPreviewText}
					onPreviewSize={(value) => (previewSize = value)}
					onPreviewWeight={(value) => (previewWeight = value)}
				/>
			</section>
		{:else if view === 'duplicates'}
			<ConflictsView families={conflictFamilies} onInspect={inspectConflict} />
		{:else}
			<SettingsView
				{theme}
				{density}
				{previewText}
				onTheme={setTheme}
				onDensity={setDensity}
				onPreviewText={setPreviewText}
			/>
		{/if}
	</main>
</div>

{#if toast}
	<div class:error={toast.tone === 'error'} class="toast" role="status" aria-live="polite">
		<Icon name={toast.tone === 'error' ? 'alert' : 'check'} size={17} />
		<span>{toast.message}</span>
		<button type="button" aria-label="Dismiss notification" onclick={() => (toast = null)}>
			<Icon name="close" size={16} />
		</button>
	</div>
{/if}

<style>
	.app-shell {
		display: grid;
		grid-template-columns: 208px minmax(0, 1fr);
		min-height: 100vh;
		color: var(--color-text);
		background: var(--color-bg);
	}

	main {
		min-width: 0;
		min-height: 100vh;
	}

	.library-workspace {
		display: grid;
		grid-template-columns: minmax(480px, 1fr) minmax(300px, 360px);
		min-height: 100vh;
		background: var(--color-surface);
	}

	.library-pane {
		min-width: 0;
		background: var(--color-surface);
	}

	.library-header {
		position: sticky;
		top: 0;
		z-index: var(--z-sticky);
		padding: 20px 24px 14px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface);
	}

	.heading-row,
	.header-actions,
	.toolbar {
		display: flex;
		align-items: center;
	}

	.heading-row {
		align-items: flex-start;
		justify-content: space-between;
		gap: 20px;
	}

	.section-label {
		margin: 0;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-weight: 650;
		letter-spacing: 0.045em;
		text-transform: uppercase;
	}

	h1 {
		margin: 4px 0 0;
		font-size: var(--text-heading);
		line-height: 1.2;
		letter-spacing: -0.03em;
		text-wrap: balance;
	}

	.catalogue-summary {
		margin: 5px 0 0;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
	}

	.header-actions {
		gap: 8px;
	}

	.icon-action,
	.primary-action,
	.filter-menu summary,
	.state-view button,
	.load-more-row button {
		display: inline-flex;
		height: 36px;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-md);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
		transition:
			background var(--motion-fast),
			color var(--motion-fast),
			transform var(--motion-fast);
	}

	.icon-action {
		width: 36px;
		padding: 0;
		border: 1px solid var(--color-border);
		color: var(--color-muted);
		background: var(--color-control);
	}

	.icon-action:hover {
		color: var(--color-text);
		background: var(--color-selected);
	}

	.primary-action {
		gap: 7px;
		padding: 0 12px;
		border: 1px solid var(--color-accent);
		color: var(--color-accent-ink);
		background: var(--color-accent);
	}

	.primary-action:hover {
		background: var(--color-accent-hover);
	}

	.icon-action:active,
	.primary-action:active,
	.filter-menu summary:active {
		transform: translateY(1px);
	}

	.toolbar {
		gap: 8px;
		margin-top: 15px;
	}

	.search-field {
		position: relative;
		display: flex;
		min-width: 220px;
		flex: 1;
		align-items: center;
		color: var(--color-muted);
	}

	.search-field > :global(svg) {
		position: absolute;
		left: 11px;
		pointer-events: none;
	}

	.search-field input {
		width: 100%;
		height: 36px;
		padding: 0 38px 0 36px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-body-sm);
	}

	.search-field input::placeholder {
		color: var(--color-muted);
		opacity: 1;
	}

	.search-field kbd {
		position: absolute;
		right: 8px;
		display: grid;
		width: 22px;
		height: 22px;
		place-items: center;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-xs);
		color: var(--color-subtle);
		background: var(--color-panel);
		font: inherit;
		font-size: var(--text-micro);
	}

	.filter-menu {
		position: relative;
		flex: none;
	}

	.filter-menu summary {
		gap: 7px;
		padding: 0 11px;
		border: 1px solid var(--color-border);
		color: var(--color-text);
		background: var(--color-control);
		list-style: none;
	}

	.filter-menu summary::-webkit-details-marker {
		display: none;
	}

	.filter-menu[open] summary,
	.filter-menu summary:hover {
		background: var(--color-selected);
	}

	.filter-panel {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
		z-index: var(--z-dropdown);
		display: grid;
		width: 292px;
		gap: 12px;
		padding: 14px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-raised);
		box-shadow: var(--shadow-floating);
	}

	.filter-panel-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		font-size: var(--text-label);
	}

	.filter-panel-head button {
		padding: 4px 6px;
		border: 0;
		border-radius: var(--radius-xs);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-micro);
		font-weight: 600;
		cursor: pointer;
	}

	.filter-panel-head button:disabled {
		cursor: default;
		opacity: 0.45;
	}

	.filter-panel > label:not(.check-filter) {
		display: grid;
		gap: 6px;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-weight: 600;
	}

	.filter-panel select {
		height: 34px;
		padding: 0 9px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-body-sm);
	}

	.check-filter {
		display: flex;
		min-height: 30px;
		align-items: center;
		gap: 9px;
		color: var(--color-text);
		font-size: var(--text-body-sm);
	}

	.check-filter input {
		width: 16px;
		height: 16px;
		accent-color: var(--color-accent);
	}

	.table-head,
	.font-row {
		display: grid;
		grid-template-columns: minmax(150px, 0.9fr) minmax(210px, 1.45fr) 86px;
		align-items: center;
		gap: 16px;
	}

	.table-head {
		height: 32px;
		padding: 0 24px;
		border-bottom: 1px solid var(--color-border);
		color: var(--color-subtle);
		background: var(--color-panel);
		font-size: var(--text-micro);
		font-weight: 650;
		letter-spacing: 0.035em;
		text-transform: uppercase;
	}

	.font-list {
		min-height: 420px;
	}

	.font-row {
		width: 100%;
		min-height: 68px;
		padding: 9px 24px;
		border: 0;
		border-bottom: 1px solid var(--color-border);
		color: var(--color-text);
		background: transparent;
		font: inherit;
		text-align: left;
		cursor: pointer;
		content-visibility: auto;
		contain-intrinsic-size: auto 68px;
		transition: background var(--motion-fast);
	}

	.font-row:hover {
		background: var(--color-hover);
	}

	.font-row.selected {
		background: var(--color-selected);
	}

	.family-cell,
	.style-cell {
		display: grid;
		min-width: 0;
		gap: 4px;
	}

	.family-cell strong {
		overflow: hidden;
		font-size: var(--text-body-sm);
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.family-cell small,
	.style-cell,
	.style-cell small {
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.family-cell small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.preview-cell {
		overflow: hidden;
		font-size: 1.25rem;
		line-height: 1.15;
		letter-spacing: -0.02em;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.style-cell {
		justify-items: end;
		font-variant-numeric: tabular-nums;
	}

	.style-cell small {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		color: var(--color-warning);
	}

	.compact .font-row {
		min-height: 52px;
		padding-top: 6px;
		padding-bottom: 6px;
		contain-intrinsic-size: auto 52px;
	}

	.compact .preview-cell {
		font-size: 1.05rem;
	}

	.skeleton-row {
		cursor: wait;
	}

	.skeleton {
		display: block;
		height: 11px;
		border-radius: var(--radius-xs);
		background: var(--color-skeleton);
		animation: skeleton-pulse 1.3s ease-in-out infinite;
		animation-delay: calc(var(--skeleton-index) * 55ms);
	}

	.family-skeleton {
		width: 68%;
	}

	.preview-skeleton {
		width: 84%;
		height: 18px;
	}

	.count-skeleton {
		width: 42px;
		justify-self: end;
	}

	.state-view {
		display: grid;
		max-width: 460px;
		place-items: center;
		margin: 64px auto;
		padding: 28px;
		text-align: center;
	}

	.state-icon {
		display: grid;
		width: 50px;
		height: 50px;
		place-items: center;
		margin-bottom: 15px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-muted);
		background: var(--color-raised);
	}

	.state-icon.error {
		color: var(--color-danger);
	}

	.state-view h2 {
		margin: 0;
		font-size: var(--text-title);
	}

	.state-view p {
		max-width: 54ch;
		margin: 8px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.55;
	}

	.state-view button,
	.load-more-row button {
		margin-top: 16px;
		padding: 0 12px;
		border: 1px solid var(--color-border);
		color: var(--color-text);
		background: var(--color-control);
	}

	.state-view button:hover,
	.load-more-row button:hover {
		background: var(--color-selected);
	}

	.load-more-row {
		display: grid;
		place-items: center;
		padding: 8px 16px 24px;
	}

	.toast {
		position: fixed;
		right: 20px;
		bottom: 20px;
		z-index: var(--z-toast);
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		max-width: min(440px, calc(100vw - 32px));
		align-items: center;
		gap: 10px;
		padding: 12px 12px 12px 14px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-success);
		background: var(--color-raised);
		box-shadow: var(--shadow-floating);
		font-size: var(--text-body-sm);
		animation: toast-in var(--motion-standard);
	}

	.toast.error {
		color: var(--color-danger);
	}

	.toast span {
		color: var(--color-text);
	}

	.toast button {
		display: grid;
		width: 32px;
		height: 32px;
		place-items: center;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		cursor: pointer;
	}

	.toast button:hover {
		color: var(--color-text);
		background: var(--color-selected);
	}

	.visually-hidden-file {
		position: fixed;
		width: 1px;
		height: 1px;
		opacity: 0;
		pointer-events: none;
	}

	@keyframes skeleton-pulse {
		50% {
			opacity: 0.45;
		}
	}

	@keyframes toast-in {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}

	@media (max-width: 1119px) {
		.library-workspace {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 819px) {
		.app-shell {
			display: block;
		}

		main {
			min-height: calc(100vh - 57px);
		}

		.library-header {
			top: 57px;
		}
	}

	@media (max-width: 700px) {
		.library-header {
			padding: 18px 16px 12px;
		}

		.heading-row {
			align-items: center;
		}

		.primary-action {
			width: 36px;
			padding: 0;
		}

		.primary-action span {
			display: none;
		}

		.table-head,
		.font-row {
			grid-template-columns: minmax(120px, 0.8fr) minmax(170px, 1.2fr);
			padding-right: 16px;
			padding-left: 16px;
		}

		.table-head span:last-child,
		.style-cell {
			display: none;
		}

		.toast {
			right: 16px;
			bottom: 16px;
			left: 16px;
		}
	}

	@media (max-width: 520px) {
		.catalogue-summary {
			max-width: 32ch;
		}

		.toolbar {
			align-items: stretch;
		}

		.filter-menu summary {
			width: 40px;
			padding: 0;
			font-size: 0;
		}

		.filter-panel {
			position: fixed;
			top: auto;
			right: 12px;
			bottom: 12px;
			left: 12px;
			width: auto;
		}

		.table-head,
		.font-row {
			grid-template-columns: 1fr;
		}

		.table-head span:nth-child(2),
		.preview-cell {
			display: none;
		}
	}
</style>
