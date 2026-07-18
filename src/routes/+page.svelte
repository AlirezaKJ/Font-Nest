<script lang="ts">
	import { onMount } from 'svelte';

	import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
	import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
	import { getConflictDestination } from '$lib/conflict-navigation';
	import AppNavigation, { type AppView } from '$lib/components/AppNavigation.svelte';
	import AppTitleBar from '$lib/components/AppTitleBar.svelte';
	import ConflictsView from '$lib/components/ConflictsView.svelte';
	import DiscoverFilterMenu, {
		type DiscoverFilterOption
	} from '$lib/components/DiscoverFilterMenu.svelte';
	import DiscoverView from '$lib/components/DiscoverView.svelte';
	import FontPreviewView from '$lib/components/FontPreviewView.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import SettingsView, {
		type DensityPreference,
		type ThemePreference
	} from '$lib/components/SettingsView.svelte';
	import { createBrowserCatalogue } from '$lib/catalogue/browser-catalogue';
	import { reorderIds, type ReorderPosition } from '$lib/reorder';
	import { isStickySurfaceElevated } from '$lib/sticky-surface';
	import { scanInstalledFonts } from '$lib/tauri/commands';

	const PAGE_SIZE = 120;
	const SKELETON_ROWS = [0, 1, 2, 3];
	const DEFAULT_PREVIEW = 'Hamburgefontsiv 0123 — Pack my box with five dozen liquor jugs.';
	const DEFAULT_SPECIMEN_SIZE = 96;
	const MAX_DETAIL_FACES = 12;
	const PREFERENCES_KEY = 'fontnest.preferences.v1';
	const GLYPH_SAMPLE = [
		'A',
		'a',
		'g',
		'R',
		'Q',
		'y',
		'ß',
		'Æ',
		'ø',
		'Ж',
		'7',
		'&',
		'@',
		'½',
		'→'
	];

	const SPACING_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'Any spacing' },
		{ value: 'proportional', label: 'Proportional' },
		{ value: 'monospaced', label: 'Monospaced' }
	];
	const STATUS_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'Any status' },
		{ value: 'conflict', label: 'Conflicts only', description: 'Families with duplicate files' }
	];
	const SORT_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'name-asc', label: 'Name A–Z' },
		{ value: 'name-desc', label: 'Name Z–A' },
		{ value: 'styles', label: 'Most styles' },
		{ value: 'faces', label: 'Most faces' }
	];

	type CatalogueMode = 'native' | 'browser';
	type SpecimenMode = 'names' | 'custom';
	type LibraryFilterKey = 'source' | 'format' | 'spacing' | 'status' | 'sort';
	type ActiveLibraryFilter = { key: LibraryFilterKey; label: string };
	type Toast = { message: string; tone: 'success' | 'error' };

	let view = $state<AppView>('library');
	let libraryControlsElement = $state<HTMLElement>();
	let libraryControlsElevated = $state(false);

	$effect(() => {
		if (view !== 'library') libraryControlsElevated = false;
	});

	let catalogue = $state<FontCatalogue | null>(null);
	let catalogueMode = $state<CatalogueMode>('browser');
	let loading = $state(true);
	let errorMessage = $state('');
	let selectedFamilyId = $state<string | null>(null);
	let search = $state('');
	let sourceFilter = $state('all');
	let formatFilter = $state('all');
	let spacingFilter = $state('all');
	let statusFilter = $state('all');
	let sortOrder = $state('name-asc');
	let specimenMode = $state<SpecimenMode>('names');
	let specimenSize = $state(DEFAULT_SPECIMEN_SIZE);
	let displayLimit = $state(PAGE_SIZE);
	let previewText = $state(DEFAULT_PREVIEW);
	let previewSize = $state(64);
	let previewWeight = $state(400);
	let theme = $state<ThemePreference>('dark');
	let density = $state<DensityPreference>('comfortable');
	let sidebarCollapsed = $state(false);
	let pinnedFamilyIds = $state<string[]>([]);
	let toast = $state<Toast | null>(null);
	let previewFileInput = $state<HTMLInputElement>();
	let toastTimer: ReturnType<typeof setTimeout> | undefined;

	let sourceOptions = $derived.by<DiscoverFilterOption[]>(() => {
		const values = [
			...new Set(catalogue?.families.flatMap((family) => family.sources) ?? [])
		].sort();
		return [
			{ value: 'all', label: 'All sources' },
			...values.map((value) => ({ value, label: value }))
		];
	});

	let formatOptions = $derived.by<DiscoverFilterOption[]>(() => {
		const values = [
			...new Set(catalogue?.families.flatMap((family) => family.formats) ?? [])
		].sort();
		return [
			{ value: 'all', label: 'All formats' },
			...values.map((value) => ({ value, label: value }))
		];
	});

	let activeFilters = $derived.by<ActiveLibraryFilter[]>(() => {
		const filters: ActiveLibraryFilter[] = [];
		if (sourceFilter !== 'all')
			filters.push({ key: 'source', label: optionLabel(sourceOptions, sourceFilter) });
		if (formatFilter !== 'all')
			filters.push({ key: 'format', label: optionLabel(formatOptions, formatFilter) });
		if (spacingFilter !== 'all')
			filters.push({ key: 'spacing', label: optionLabel(SPACING_OPTIONS, spacingFilter) });
		if (statusFilter !== 'all')
			filters.push({ key: 'status', label: optionLabel(STATUS_OPTIONS, statusFilter) });
		if (sortOrder !== 'name-asc')
			filters.push({ key: 'sort', label: optionLabel(SORT_OPTIONS, sortOrder) });
		return filters;
	});

	let hasResettableState = $derived(
		Boolean(search) ||
			activeFilters.length > 0 ||
			specimenMode !== 'names' ||
			specimenSize !== DEFAULT_SPECIMEN_SIZE
	);

	let filteredFamilies = $derived.by(() => {
		const terms = search
			.toLocaleLowerCase()
			.split(/\s+/)
			.map((term) => term.trim())
			.filter(Boolean);

		const matching = (catalogue?.families ?? []).filter((family) => {
			const searchable = [family.name, ...family.styles, ...family.sources, ...family.formats]
				.join(' ')
				.toLocaleLowerCase();
			return (
				terms.every((term) => searchable.includes(term)) &&
				(sourceFilter === 'all' || family.sources.includes(sourceFilter)) &&
				(formatFilter === 'all' || family.formats.includes(formatFilter)) &&
				(spacingFilter === 'all' ||
					family.monospaced === (spacingFilter === 'monospaced')) &&
				(statusFilter === 'all' || family.hasConflict)
			);
		});

		return matching.sort((left, right) => {
			if (sortOrder === 'name-desc') return right.name.localeCompare(left.name);
			if (sortOrder === 'styles')
				return right.faceCount - left.faceCount || left.name.localeCompare(right.name);
			if (sortOrder === 'faces')
				return right.fileCount - left.fileCount || left.name.localeCompare(right.name);
			return left.name.localeCompare(right.name);
		});
	});

	let renderedFamilies = $derived(filteredFamilies.slice(0, displayLimit));
	let selectedFamily = $derived.by(() => {
		const selected =
			catalogue?.families.find((family) => family.id === selectedFamilyId) ?? null;
		if (view === 'preview') return selected;
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
	let pinnedFamilies = $derived.by(() =>
		pinnedFamilyIds
			.map((familyId) => catalogue?.families.find((family) => family.id === familyId))
			.filter((family): family is FontFamilySummary => Boolean(family))
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
				focusSearch();
			} else if (event.key === 'Escape' && search) {
				search = '';
				displayLimit = PAGE_SIZE;
				focusSearch();
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
				sidebarCollapsed?: boolean;
				pinnedFamilyIds?: unknown;
			};
			if (saved.theme && ['system', 'light', 'dark'].includes(saved.theme))
				theme = saved.theme;
			if (saved.density && ['comfortable', 'compact'].includes(saved.density)) {
				density = saved.density;
			}
			if (saved.previewText?.trim()) previewText = saved.previewText;
			if (typeof saved.sidebarCollapsed === 'boolean') {
				sidebarCollapsed = saved.sidebarCollapsed;
			}
			if (Array.isArray(saved.pinnedFamilyIds)) {
				pinnedFamilyIds = [
					...new Set(
						saved.pinnedFamilyIds.filter(
							(value): value is string => typeof value === 'string'
						)
					)
				];
			}
		} catch {
			localStorage.removeItem(PREFERENCES_KEY);
		}
	}

	function savePreferences() {
		localStorage.setItem(
			PREFERENCES_KEY,
			JSON.stringify({ theme, density, previewText, sidebarCollapsed, pinnedFamilyIds })
		);
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

	function toggleSidebar() {
		sidebarCollapsed = !sidebarCollapsed;
		savePreferences();
	}

	async function refreshCatalogue() {
		loading = true;
		errorMessage = '';
		const isNative = '__TAURI_INTERNALS__' in window;
		catalogueMode = isNative ? 'native' : 'browser';

		if (!isNative) {
			catalogue = createBrowserCatalogue();
			selectedFamilyId = null;
			previewWeight = nearestWeight(catalogue.families[0]?.weights ?? [400], 400);
			loading = false;
			return;
		}

		try {
			catalogue = await scanInstalledFonts();
			selectedFamilyId = null;
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

	function openFamilyPreview(familyId: string) {
		const family = catalogue?.families.find((candidate) => candidate.id === familyId);
		if (!family) return;

		selectFamily(familyId);
		view = 'preview';
		if (!pinnedFamilyIds.includes(familyId)) {
			pinnedFamilyIds = [...pinnedFamilyIds, familyId];
			savePreferences();
			showToast(`${family.name} added to saved previews.`, 'success');
		}
	}

	function closeFamilyPreview(familyId: string) {
		if (!pinnedFamilyIds.includes(familyId)) return;

		const family = catalogue?.families.find((candidate) => candidate.id === familyId);
		pinnedFamilyIds = pinnedFamilyIds.filter((candidate) => candidate !== familyId);
		if (view === 'preview' && selectedFamilyId === familyId) view = 'library';
		savePreferences();
		showToast(`${family?.name ?? 'Preview'} closed.`, 'success');
	}

	function reorderPinnedFamily(
		draggedFamilyId: string,
		targetFamilyId: string,
		position: ReorderPosition
	) {
		const reordered = reorderIds(pinnedFamilyIds, draggedFamilyId, targetFamilyId, position);
		if (reordered.every((familyId, index) => familyId === pinnedFamilyIds[index])) return;
		pinnedFamilyIds = reordered;
		savePreferences();
	}

	function toggleSelectedFamilyPinned() {
		const family = selectedFamily;
		if (!family) return;
		const isPinned = pinnedFamilyIds.includes(family.id);
		pinnedFamilyIds = isPinned
			? pinnedFamilyIds.filter((familyId) => familyId !== family.id)
			: [...pinnedFamilyIds, family.id];
		savePreferences();
		showToast(
			isPinned
				? `${family.name} removed from saved previews.`
				: `${family.name} added to saved previews.`,
			'success'
		);
	}

	function reviewConflict(familyId: string) {
		selectFamily(familyId);
		view = getConflictDestination('review');
	}

	function inspectConflict(familyId: string) {
		selectFamily(familyId);
		view = getConflictDestination('inspect');
	}

	function handleLibraryScroll(event: Event) {
		const scrollContainer = event.currentTarget as HTMLElement;
		const elevated = isStickySurfaceElevated(
			scrollContainer.scrollTop,
			libraryControlsElement?.offsetTop ?? Number.POSITIVE_INFINITY
		);
		if (elevated !== libraryControlsElevated) libraryControlsElevated = elevated;
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
		document.querySelectorAll<HTMLButtonElement>('.specimen-toggle')[nextIndex]?.focus();
	}

	function updateSearch(value: string) {
		search = value;
		displayLimit = PAGE_SIZE;
	}

	function updateGlobalSearch(value: string) {
		view = 'library';
		updateSearch(value);
	}

	function focusSearch() {
		requestAnimationFrame(() => {
			document.querySelector<HTMLInputElement>('[data-font-search]')?.focus();
		});
	}

	function updateFilter(key: LibraryFilterKey, value: string) {
		if (key === 'source') sourceFilter = value;
		if (key === 'format') formatFilter = value;
		if (key === 'spacing') spacingFilter = value;
		if (key === 'status') statusFilter = value;
		if (key === 'sort') sortOrder = value;
		displayLimit = PAGE_SIZE;
	}

	function clearFilter(key: LibraryFilterKey) {
		updateFilter(key, key === 'sort' ? 'name-asc' : 'all');
	}

	function clearFilters() {
		sourceFilter = 'all';
		formatFilter = 'all';
		spacingFilter = 'all';
		statusFilter = 'all';
		sortOrder = 'name-asc';
		displayLimit = PAGE_SIZE;
	}

	function resetAll() {
		search = '';
		clearFilters();
		specimenMode = 'names';
		specimenSize = DEFAULT_SPECIMEN_SIZE;
	}

	function optionLabel(options: DiscoverFilterOption[], value: string): string {
		return options.find((option) => option.value === value)?.label ?? value;
	}

	function toggleFamily(familyId: string) {
		if (selectedFamilyId === familyId) {
			selectedFamilyId = null;
			return;
		}
		selectFamily(familyId);
	}

	function safeFontStack(name: string): string {
		return `"${name.replace(/["\\;\n\r]/g, '')}", system-ui, sans-serif`;
	}

	function familyPreviewStyle(family: FontFamilySummary): string {
		return `font-family: ${safeFontStack(family.name)}; font-weight: ${nearestWeight(family.weights, 400)};`;
	}

	function faceSpecimenStyle(family: FontFamilySummary, weight: number, style: string): string {
		return `font-family: ${safeFontStack(family.name)}; font-weight: ${weight}; font-style: ${style === 'italic' ? 'italic' : 'normal'};`;
	}

	function specimenText(family: FontFamilySummary): string {
		return specimenMode === 'names' ? family.name : previewText.trim() || family.name;
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

<AppTitleBar
	{search}
	{loading}
	{theme}
	settingsActive={view === 'settings'}
	onSearch={updateGlobalSearch}
	onNavigate={(nextView) => (view = nextView)}
	onToggleTheme={toggleTheme}
	onRefresh={() => void refreshCatalogue()}
	onPreview={openPreviewFilePicker}
/>

<div
	class:compact={density === 'compact'}
	class:sidebar-collapsed={sidebarCollapsed}
	class="app-shell"
>
	<AppNavigation
		{view}
		familyCount={catalogue?.familyCount ?? 0}
		conflictCount={catalogue?.conflictCount ?? 0}
		{loading}
		mode={catalogueMode}
		collapsed={sidebarCollapsed}
		{pinnedFamilies}
		activeFamilyId={view === 'preview' ? (selectedFamily?.id ?? null) : null}
		onNavigate={(nextView) => (view = nextView)}
		onOpenPreview={openFamilyPreview}
		onClosePreview={closeFamilyPreview}
		onReorderPreview={reorderPinnedFamily}
		onToggle={toggleSidebar}
		onRefresh={() => void refreshCatalogue()}
	/>

	<main id="main-content">
		{#if view === 'library'}
			<section
				class="library-view"
				aria-labelledby="library-title"
				onscroll={handleLibraryScroll}
			>
				<header class="library-header">
					<div class="header-lead">
						<h1 id="library-title">Your fonts</h1>
						<p class="catalogue-summary">
							{#if catalogue}
								{catalogue.familyCount.toLocaleString()} families · {catalogue.faceCount.toLocaleString()}
								faces{#if catalogueMode === 'native'}
									· scanned in {catalogue.scanDurationMs.toLocaleString()}
									ms{/if}
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
							class="primary-action"
							onclick={openPreviewFilePicker}
						>
							<svg
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="1.7"
								stroke-linecap="round"
								stroke-linejoin="round"
								aria-hidden="true"
								width="16"
								height="16"
							>
								<path d="M12 15.5V4.75M8.25 8.5 12 4.75l3.75 3.75" />
								<path
									d="M5 14.5v3.25A1.75 1.75 0 0 0 6.75 19.5h10.5A1.75 1.75 0 0 0 19 17.75V14.5"
								/>
							</svg>
							<span>Preview a font</span>
						</button>
					</div>
				</header>

				<section
					bind:this={libraryControlsElement}
					class:is-elevated={libraryControlsElevated}
					class="library-controls sticky-control-surface"
					aria-label="Library controls"
				>
					<div class="primary-toolbar">
						<label class="search-control">
							<span>Search</span>
							<Icon name="search" size={15} />
							<input
								data-font-search
								type="search"
								placeholder="Families, styles, sources"
								value={search}
								oninput={(event) => updateSearch(event.currentTarget.value)}
							/>
						</label>
						<div class="filter-strip">
							<DiscoverFilterMenu
								id="library-source"
								label="Source"
								value={sourceFilter}
								options={sourceOptions}
								onChange={(value) => updateFilter('source', value)}
							/>
							<DiscoverFilterMenu
								id="library-format"
								label="Format"
								value={formatFilter}
								options={formatOptions}
								onChange={(value) => updateFilter('format', value)}
							/>
							<DiscoverFilterMenu
								id="library-spacing"
								label="Spacing"
								value={spacingFilter}
								options={SPACING_OPTIONS}
								onChange={(value) => updateFilter('spacing', value)}
							/>
							<DiscoverFilterMenu
								id="library-status"
								label="Status"
								value={statusFilter}
								options={STATUS_OPTIONS}
								onChange={(value) => updateFilter('status', value)}
							/>
							<DiscoverFilterMenu
								id="library-sort"
								label="Sort"
								value={sortOrder}
								options={SORT_OPTIONS}
								onChange={(value) => updateFilter('sort', value)}
							/>
						</div>
					</div>

					<div class="specimen-toolbar">
						<label class="preview-text-control">
							<span>Preview text</span>
							<Icon name="font" size={15} />
							<input
								type="text"
								value={previewText}
								placeholder="Type a shared specimen"
								disabled={specimenMode === 'names'}
								oninput={(event) => setPreviewText(event.currentTarget.value)}
							/>
						</label>
						<div class="specimen-modes" role="group" aria-label="Specimen text mode">
							<button
								type="button"
								class:active={specimenMode === 'names'}
								aria-pressed={specimenMode === 'names'}
								onclick={() => (specimenMode = 'names')}>Names</button
							>
							<button
								type="button"
								class:active={specimenMode === 'custom'}
								aria-pressed={specimenMode === 'custom'}
								onclick={() => (specimenMode = 'custom')}>Your text</button
							>
						</div>
						<label class="size-control">
							<span>Size</span>
							<input
								type="range"
								min="48"
								max="148"
								step="4"
								value={specimenSize}
								oninput={(event) =>
									(specimenSize = Number(event.currentTarget.value))}
							/>
							<output>{specimenSize}px</output>
						</label>
						<div class="active-filter-summary" aria-live="polite">
							{#if activeFilters.length}
								{#each activeFilters as filter (filter.key)}
									<button
										type="button"
										aria-label={`Remove ${filter.label} filter`}
										onclick={() => clearFilter(filter.key)}
									>
										{filter.label}<Icon name="close" size={12} />
									</button>
								{/each}
							{:else}
								<span>All families</span>
							{/if}
						</div>
						<button
							type="button"
							class="reset-action"
							disabled={!hasResettableState}
							onclick={resetAll}>Reset all</button
						>
					</div>
				</section>

				<div class="specimen-feed" style={`--specimen-size: ${specimenSize}px`}>
					<div class="catalogue-heading">
						<strong>{filteredFamilies.length.toLocaleString()} families</strong>
						<span>Rendered in the fonts installed on this computer</span>
					</div>

					{#if loading && !catalogue}
						<div class="specimen-list" aria-label="Loading font families">
							{#each SKELETON_ROWS as row (row)}
								<div class="specimen-entry loading-entry" aria-hidden="true">
									<div class="loading-meta">
										<span></span><span></span><span></span>
									</div>
									<div class="specimen-skeleton">
										<span></span><span></span><span></span>
									</div>
								</div>
							{/each}
						</div>
					{:else if errorMessage}
						<div class="catalogue-state" role="alert">
							<div class="state-icon error"><Icon name="alert" size={20} /></div>
							<h2>Catalogue scan did not finish</h2>
							<p>{errorMessage}</p>
							<button type="button" onclick={() => void refreshCatalogue()}
								>Scan again</button
							>
						</div>
					{:else if !catalogue?.familyCount}
						<div class="catalogue-state">
							<div class="state-icon"><Icon name="font" size={20} /></div>
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
						<div class="catalogue-state">
							<div class="state-icon"><Icon name="search" size={20} /></div>
							<h2>No families match</h2>
							<p>Try a shorter search, or remove one of the active filters.</p>
							<button
								type="button"
								onclick={() => {
									search = '';
									clearFilters();
								}}>Clear search and filters</button
							>
						</div>
					{:else}
						<div class="specimen-list" aria-label="Font families">
							{#each renderedFamilies as family, index (family.id)}
								<article
									class:selected={selectedFamilyId === family.id}
									class="specimen-entry"
								>
									<button
										type="button"
										class="specimen-toggle"
										aria-expanded={selectedFamilyId === family.id}
										aria-controls={`family-details-${family.id}`}
										onclick={() => toggleFamily(family.id)}
										onkeydown={(event) => handleRowKeydown(event, index)}
									>
										<span class="family-line">
											<strong>{family.name}</strong>
											<span class="meta-source"
												>{family.sources.join(' · ')}</span
											>
											<span class="meta-format"
												>{family.formats.join(' · ')}</span
											>
											<span class="meta-count">
												{family.faceCount}
												{family.faceCount === 1 ? 'style' : 'styles'}
											</span>
											<span class="meta-spacing"
												>{family.monospaced
													? 'Monospaced'
													: 'Proportional'}</span
											>
											{#if family.hasConflict}
												<span class="conflict-label"
													><Icon name="alert" size={12} /> Conflict</span
												>
											{/if}
											<span class="open-label">
												{selectedFamilyId === family.id
													? 'Close'
													: 'Open family'}
												<Icon name="chevron" size={13} />
											</span>
										</span>
										<span
											class="specimen-canvas"
											style={familyPreviewStyle(family)}
										>
											<span class="specimen-text">{specimenText(family)}</span
											>
										</span>
									</button>

									{#if selectedFamilyId === family.id}
										<section
											id={`family-details-${family.id}`}
											class="family-details"
										>
											<div class="detail-heading">
												<div>
													<strong
														>{family.faceCount} faces · {family.fileCount}
														files</strong
													>
													<p>
														{family.formats.join(' · ')} · {family.monospaced
															? 'Monospaced'
															: 'Proportional'}
													</p>
												</div>
												<div class="detail-actions">
													{#if family.hasConflict}
														<button
															type="button"
															class="detail-action ghost"
															onclick={() =>
																reviewConflict(family.id)}
														>
															<Icon name="alert" size={15} /> Review conflict
														</button>
													{/if}
													<button
														type="button"
														class:pinned={pinnedFamilyIds.includes(
															family.id
														)}
														class="detail-action"
														onclick={() => openFamilyPreview(family.id)}
													>
														<Icon name="bookmark" size={15} />
														{pinnedFamilyIds.includes(family.id)
															? 'Open preview'
															: 'Save & preview'}
													</button>
												</div>
											</div>

											<ul class="face-list">
												{#each family.faces.slice(0, MAX_DETAIL_FACES) as face (face.id)}
													<li>
														<span class="face-meta">
															<strong>{face.styleName}</strong>
															<small>{face.fileName}</small>
														</span>
														<span
															class="face-specimen"
															style={faceSpecimenStyle(
																family,
																face.weight,
																face.style
															)}>{face.styleName}</span
														>
													</li>
												{/each}
											</ul>
											{#if family.faces.length > MAX_DETAIL_FACES}
												<p class="more-faces">
													+{family.faces.length - MAX_DETAIL_FACES} more faces
												</p>
											{/if}

											<div class="glyph-sample">
												<h3>Character sample</h3>
												<div
													class="glyphs"
													style={`font-family: ${safeFontStack(family.name)}`}
												>
													{#each GLYPH_SAMPLE as glyph (glyph)}<span
															>{glyph}</span
														>{/each}
												</div>
											</div>
										</section>
									{/if}
								</article>
							{/each}
						</div>

						{#if renderedFamilies.length < filteredFamilies.length}
							<div class="load-more-row">
								<button type="button" onclick={() => (displayLimit += PAGE_SIZE)}>
									{Math.min(
										PAGE_SIZE,
										filteredFamilies.length - renderedFamilies.length
									)} more
								</button>
								<span
									>{renderedFamilies.length.toLocaleString()} of {filteredFamilies.length.toLocaleString()}</span
								>
							</div>
						{/if}
					{/if}
				</div>
			</section>
		{:else if view === 'discover'}
			<DiscoverView
				installedFamilyNames={catalogue?.families.map((family) => family.name) ?? []}
				onInstalled={refreshCatalogue}
				onToast={showToast}
			/>
		{:else if view === 'duplicates'}
			<ConflictsView families={conflictFamilies} onInspect={inspectConflict} />
		{:else if view === 'preview'}
			<FontPreviewView
				family={selectedFamily}
				{previewText}
				{previewSize}
				{previewWeight}
				pinned={selectedFamily ? pinnedFamilyIds.includes(selectedFamily.id) : false}
				onBack={() => (view = 'library')}
				onTogglePinned={toggleSelectedFamilyPinned}
				onPreviewText={setPreviewText}
				onPreviewSize={(value) => (previewSize = value)}
				onPreviewWeight={(value) => (previewWeight = value)}
			/>
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
		--titlebar-height: 48px;
		--app-content-height: calc(100dvh - var(--titlebar-height));

		display: grid;
		height: var(--app-content-height);
		grid-template-columns: 208px minmax(0, 1fr);
		min-height: 0;
		overflow: hidden;
		color: var(--color-text);
		background: var(--color-bg);
		transition: grid-template-columns var(--motion-standard);
	}

	.app-shell.sidebar-collapsed {
		grid-template-columns: 56px minmax(0, 1fr);
	}

	main {
		min-width: 0;
		min-height: 0;
		overflow: auto;
	}

	.library-view {
		display: flex;
		width: 100%;
		min-width: 0;
		height: 100%;
		min-height: 0;
		flex-direction: column;
		overflow-x: hidden;
		overflow-y: auto;
		background: var(--color-surface);
	}

	.library-header {
		display: flex;
		width: 100%;
		min-width: 0;
		flex: none;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2xl);
		padding: 18px 24px 14px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface);
	}

	.header-lead {
		min-width: 0;
	}

	h1,
	h2,
	p {
		margin-top: 0;
	}

	.section-label {
		margin: 0;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-weight: 650;
		letter-spacing: 0.045em;
		text-transform: uppercase;
	}

	.library-header h1 {
		margin: 3px 0 0;
		font-size: var(--text-heading);
		line-height: 1.15;
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
		display: flex;
		flex: none;
		gap: var(--space-sm);
	}

	.primary-action {
		display: inline-flex;
		height: 36px;
		align-items: center;
		justify-content: center;
		gap: 7px;
		padding: 0 12px;
		border: 1px solid var(--color-accent);
		border-radius: var(--radius-md);
		color: var(--color-accent-ink);
		background: var(--color-accent);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
		transition:
			background var(--motion-fast),
			transform var(--motion-fast);
	}

	.primary-action:hover {
		background: var(--color-accent-hover);
	}

	.primary-action:active {
		transform: translateY(1px);
	}

	/* Controls — shared vocabulary with the Discover view */
	.library-controls {
		position: sticky;
		top: 0;
		z-index: var(--z-sticky);
		width: 100%;
		min-width: 0;
		flex: none;
	}

	.primary-toolbar {
		display: grid;
		width: 100%;
		min-width: 0;
		grid-template-columns: minmax(260px, 0.85fr) minmax(0, 1.55fr);
		align-items: center;
		gap: var(--space-md);
		padding: 10px 24px;
		border-bottom: 1px solid var(--color-border);
	}

	.search-control,
	.preview-text-control {
		display: grid;
		min-width: 0;
		grid-template-columns: auto auto minmax(0, 1fr);
		align-items: center;
		gap: var(--space-sm);
		padding-left: 10px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-subtle);
		background: var(--color-control);
		font-size: var(--text-micro);
	}

	.search-control:focus-within,
	.preview-text-control:focus-within {
		border-color: var(--color-focus);
	}

	.search-control input,
	.preview-text-control input {
		width: 100%;
		height: 38px;
		min-width: 0;
		border: 0;
		outline: 0;
		color: var(--color-text);
		background: transparent;
		font-size: var(--text-label);
	}

	.search-control input::placeholder,
	.preview-text-control input::placeholder {
		color: var(--color-muted);
	}

	.preview-text-control input:disabled {
		color: var(--color-subtle);
	}

	.filter-strip {
		display: flex;
		width: 100%;
		min-width: 0;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-sm);
	}

	.filter-strip :global(.filter-control) {
		min-width: 0;
		flex: 1 1 0;
	}

	.specimen-toolbar {
		display: flex;
		width: 100%;
		min-width: 0;
		align-items: center;
		gap: var(--space-md);
		padding: 10px 24px;
		border-bottom: 1px solid var(--color-border);
	}

	.preview-text-control {
		width: min(340px, 30vw);
		flex: none;
	}

	.specimen-modes {
		display: inline-flex;
		flex: none;
		padding: 2px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-control);
	}

	.specimen-modes button {
		height: 32px;
		padding: 0 10px;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.specimen-modes button:hover {
		color: var(--color-text);
	}

	.specimen-modes button.active {
		color: var(--color-text);
		background: var(--color-selected);
	}

	.size-control {
		display: grid;
		flex: none;
		grid-template-columns: auto 96px 48px;
		align-items: center;
		gap: var(--space-sm);
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.size-control input {
		accent-color: var(--color-accent);
	}

	.size-control output {
		color: var(--color-muted);
		font-size: var(--text-label);
		font-variant-numeric: tabular-nums;
	}

	.active-filter-summary {
		display: flex;
		min-width: 0;
		flex: 1;
		align-items: center;
		justify-content: flex-end;
		gap: 6px;
		overflow-x: auto;
	}

	.active-filter-summary > span {
		color: var(--color-subtle);
		font-size: var(--text-label);
		white-space: nowrap;
	}

	.active-filter-summary button {
		display: inline-flex;
		height: 28px;
		flex: none;
		align-items: center;
		gap: 5px;
		padding: 0 8px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-muted);
		background: var(--color-control);
		font-size: var(--text-micro);
		cursor: pointer;
	}

	.active-filter-summary button:hover {
		color: var(--color-text);
		border-color: var(--color-subtle);
	}

	.reset-action {
		height: 34px;
		flex: none;
		padding: 0;
		border: 0;
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.reset-action:hover:not(:disabled) {
		color: var(--color-text);
	}

	.reset-action:disabled {
		opacity: 0.45;
	}

	/* Specimen feed */
	.specimen-feed {
		container-type: inline-size;
		width: 100%;
		min-width: 0;
		min-height: 0;
		flex: none;
		overflow: visible;
		background: var(--color-surface);
	}

	.catalogue-heading {
		display: flex;
		height: 40px;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-lg);
		padding: 0 24px;
		border-bottom: 1px solid var(--color-border);
		color: var(--color-subtle);
		background: var(--color-panel);
		font-size: var(--text-micro);
	}

	.catalogue-heading strong {
		color: var(--color-text);
		font-size: var(--text-label);
		font-variant-numeric: tabular-nums;
	}

	.specimen-entry {
		content-visibility: auto;
		contain-intrinsic-size: auto 230px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface);
		transition: background var(--motion-fast);
	}

	.specimen-entry.selected {
		content-visibility: visible;
		background: color-mix(in srgb, var(--color-selected) 34%, var(--color-surface));
	}

	.specimen-toggle {
		display: block;
		width: 100%;
		padding: 0;
		border: 0;
		color: var(--color-text);
		background: transparent;
		text-align: left;
		cursor: pointer;
	}

	.specimen-toggle:hover {
		background: color-mix(in srgb, var(--color-hover) 46%, transparent);
	}

	.family-line {
		display: flex;
		min-height: 45px;
		align-items: center;
		gap: clamp(10px, 1.4vw, 22px);
		padding: 0 24px;
		overflow: hidden;
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.family-line > strong {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		color: var(--color-text);
		font-size: var(--text-label);
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.family-line > span {
		flex: none;
		white-space: nowrap;
	}

	.conflict-label {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		color: var(--color-warning);
	}

	.open-label {
		display: inline-flex;
		align-items: center;
		margin-left: auto;
		gap: 6px;
		color: var(--color-muted);
		font-weight: 650;
	}

	.open-label :global(svg) {
		transition: transform var(--motion-fast);
	}

	.selected .open-label :global(svg) {
		transform: rotate(90deg);
	}

	.specimen-canvas {
		display: flex;
		min-height: calc(var(--specimen-size) * 1.42 + 16px);
		align-items: center;
		padding: 14px 24px;
		overflow: hidden;
	}

	.specimen-text {
		display: block;
		max-width: 100%;
		font-size: var(--specimen-size);
		font-kerning: normal;
		font-optical-sizing: auto;
		line-height: 1.14;
		letter-spacing: -0.035em;
		white-space: nowrap;
	}

	.compact .specimen-canvas {
		min-height: calc(var(--specimen-size) * 1.15 + 14px);
		padding-block: 12px;
	}

	.compact .specimen-text {
		font-size: calc(var(--specimen-size) * 0.72);
	}

	/* Loading skeleton */
	.loading-entry {
		padding-bottom: 8px;
	}

	.loading-meta {
		display: flex;
		height: 45px;
		align-items: center;
		gap: 16px;
		padding: 0 24px;
	}

	.loading-meta span {
		width: 110px;
		height: 9px;
		border-radius: var(--radius-xs);
		background: var(--color-skeleton);
	}

	.specimen-skeleton {
		display: flex;
		width: min(900px, 82%);
		align-items: center;
		gap: 10px;
		margin-left: 24px;
	}

	.specimen-skeleton span {
		height: clamp(48px, 6vw, 78px);
		border-radius: var(--radius-sm);
		background: var(--color-skeleton);
		animation: skeleton-pulse 1.25s ease-in-out infinite alternate;
	}

	.specimen-skeleton span:nth-child(1) {
		width: 42%;
	}

	.specimen-skeleton span:nth-child(2) {
		width: 27%;
	}

	.specimen-skeleton span:nth-child(3) {
		width: 18%;
	}

	/* Inline family detail (replaces the sidebar inspector) */
	.family-details {
		padding: 18px 24px 22px;
		border-top: 1px solid var(--color-border);
		background: color-mix(in srgb, var(--color-panel) 62%, transparent);
	}

	.detail-heading {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-xl);
	}

	.detail-heading strong {
		font-size: var(--text-label);
	}

	.detail-heading p {
		margin: 4px 0 0;
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.detail-actions {
		display: flex;
		flex: none;
		gap: var(--space-sm);
	}

	.detail-action {
		display: inline-flex;
		height: 34px;
		align-items: center;
		gap: 7px;
		padding: 0 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		white-space: nowrap;
		cursor: pointer;
		transition:
			background var(--motion-fast),
			border-color var(--motion-fast),
			color var(--motion-fast);
	}

	.detail-action:hover {
		background: var(--color-selected);
	}

	.detail-action.ghost {
		color: var(--color-warning);
		background: transparent;
	}

	.detail-action.ghost:hover {
		background: color-mix(in srgb, var(--color-warning) 10%, transparent);
	}

	.detail-action.pinned {
		border-color: color-mix(in srgb, var(--color-accent) 58%, var(--color-border));
		background: color-mix(in srgb, var(--color-accent) 8%, var(--color-control));
	}

	.detail-action.pinned :global(svg) {
		fill: currentColor;
	}

	.face-list {
		display: grid;
		margin: 14px 0 0;
		padding: 0;
		list-style: none;
		border-top: 1px solid var(--color-border);
	}

	.face-list li {
		display: grid;
		min-height: 56px;
		grid-template-columns: minmax(130px, 220px) minmax(0, 1fr);
		align-items: center;
		gap: 20px;
		padding: 10px 0;
		overflow: hidden;
		border-bottom: 1px solid var(--color-border);
	}

	.face-meta {
		display: grid;
		min-width: 0;
		gap: 2px;
	}

	.face-meta strong {
		font-size: var(--text-body-sm);
		font-weight: 650;
	}

	.face-meta small {
		overflow: hidden;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.face-specimen {
		font-size: clamp(24px, 3.6vw, 42px);
		line-height: 1.2;
		letter-spacing: -0.02em;
		white-space: nowrap;
	}

	.more-faces {
		margin: 12px 0 0;
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.glyph-sample {
		margin-top: 20px;
	}

	.glyph-sample h3 {
		margin: 0 0 10px;
		font-size: var(--text-label);
		font-weight: 650;
	}

	.glyphs {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(56px, 1fr));
		border-top: 1px solid var(--color-border);
		border-left: 1px solid var(--color-border);
	}

	.glyphs span {
		display: grid;
		aspect-ratio: 1;
		place-items: center;
		border-right: 1px solid var(--color-border);
		border-bottom: 1px solid var(--color-border);
		font-size: var(--text-title);
	}

	/* Empty / error states */
	.catalogue-state {
		display: grid;
		min-height: 360px;
		place-items: center;
		align-content: center;
		gap: var(--space-sm);
		padding: 32px;
		text-align: center;
	}

	.state-icon {
		display: grid;
		width: 40px;
		height: 40px;
		place-items: center;
		margin-bottom: 4px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-muted);
		background: var(--color-panel);
	}

	.state-icon.error {
		color: var(--color-danger);
	}

	.catalogue-state h2 {
		margin-bottom: 0;
		font-size: var(--text-heading-sm);
	}

	.catalogue-state p {
		max-width: 48ch;
		margin-bottom: var(--space-sm);
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	.catalogue-state button {
		display: inline-flex;
		min-height: 36px;
		align-items: center;
		justify-content: center;
		padding: 0 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.catalogue-state button:hover {
		background: var(--color-selected);
	}

	.load-more-row {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-md);
		padding: 18px 24px 26px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.load-more-row button {
		display: inline-flex;
		min-height: 36px;
		align-items: center;
		justify-content: center;
		padding: 0 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.load-more-row button:hover {
		background: var(--color-selected);
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

	/* Container queries — collapse meta as the feed narrows */
	@container (max-width: 940px) {
		.meta-format,
		.meta-spacing {
			display: none;
		}

		.specimen-text {
			font-size: min(var(--specimen-size), 13cqi);
		}
	}

	@container (max-width: 680px) {
		.meta-source,
		.meta-count {
			display: none;
		}
	}

	@media (max-width: 819px) {
		.app-shell {
			display: block;
		}

		main {
			height: calc(var(--app-content-height) - 57px);
			min-height: 0;
		}
	}

	@media (max-width: 700px) {
		.library-header {
			padding: 16px 16px 12px;
		}

		.primary-action {
			width: 36px;
			padding: 0;
		}

		.primary-action span {
			display: none;
		}

		.primary-toolbar,
		.specimen-toolbar {
			padding-inline: 16px;
		}

		.primary-toolbar {
			grid-template-columns: 1fr;
		}

		.filter-strip {
			justify-content: flex-start;
			overflow-x: auto;
		}

		.filter-strip :global(.filter-control) {
			min-width: 132px;
			flex: 0 0 auto;
		}

		.specimen-toolbar {
			flex-wrap: wrap;
		}

		.preview-text-control {
			width: auto;
			flex: 1 1 240px;
		}

		.active-filter-summary {
			display: none;
		}

		.catalogue-heading,
		.family-line,
		.specimen-canvas,
		.family-details {
			padding-inline: 16px;
		}

		.detail-heading {
			align-items: stretch;
			flex-direction: column;
			gap: var(--space-md);
		}

		.face-list li {
			grid-template-columns: 1fr;
			gap: 6px;
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

		.size-control {
			grid-template-columns: auto minmax(80px, 1fr) 44px;
			width: 100%;
		}

		.catalogue-heading > span {
			display: none;
		}
	}

	.library-header {
		display: flex;
		width: 100%;
		min-width: 0;
		flex: none;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2xl);
		padding: 18px 24px 14px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface);
	}

	.header-lead {
		min-width: 0;
	}

	.library-header h1 {
		margin: 0;
		font-size: var(--text-heading);
		line-height: 1.15;
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
		display: flex;
		flex: none;
		gap: var(--space-sm);
	}

	.primary-action {
		display: inline-flex;
		height: 36px;
		align-items: center;
		justify-content: center;
		gap: 7px;
		padding: 0 12px;
		border: 1px solid var(--color-accent);
		border-radius: var(--radius-md);
		color: var(--color-accent-ink);
		background: var(--color-accent);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
		transition:
			background var(--motion-fast),
			transform var(--motion-fast);
	}

	.primary-action:hover {
		background: var(--color-accent-hover);
	}

	.primary-action:active {
		transform: translateY(1px);
	}
</style>
