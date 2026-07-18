<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';

	import type { GoogleFontFamilyDetails } from '$lib/bindings/GoogleFontFamilyDetails';
	import type { GoogleFontFamilySummary } from '$lib/bindings/GoogleFontFamilySummary';
	import type { GoogleFontPage } from '$lib/bindings/GoogleFontPage';
	import { KeyedTaskQueue, pickPreviewEvictionCandidate } from '$lib/discover/preview-queue';
	import { activateInstalledGoogleFont } from '$lib/fonts/session-fonts';
	import { isStickySurfaceElevated } from '$lib/sticky-surface';
	import {
		getGoogleFontDetails,
		installGoogleFont,
		listGoogleFonts,
		prepareGoogleFontPreview
	} from '$lib/tauri/commands';

	import DiscoverFilterMenu, { type DiscoverFilterOption } from './DiscoverFilterMenu.svelte';
	import Icon from './Icon.svelte';

	type FilterKey = 'category' | 'subset' | 'technology' | 'availability' | 'sort';
	type SpecimenMode = 'names' | 'custom';
	type PreviewStatus = 'idle' | 'loading' | 'loaded' | 'error';
	type LoadedPreview = { face: FontFace; fontFamily: string };
	type ActiveFilter = { key: FilterKey; label: string };

	const PAGE_SIZE = 40;
	const MAX_LOADED_PREVIEWS = 24;
	const PREVIEW_CONCURRENCY = 3;
	const DEFAULT_SPECIMEN_SIZE = 112;
	const DEFAULT_CUSTOM_TEXT = 'Hamburgefontsiv 0123';
	const SKELETON_ROWS = [0, 1, 2, 3];

	const CATEGORY_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'All categories' },
		{ value: 'sans-serif', label: 'Sans serif' },
		{ value: 'serif', label: 'Serif' },
		{ value: 'monospace', label: 'Monospace' },
		{ value: 'display', label: 'Display' },
		{ value: 'handwriting', label: 'Handwriting' }
	];
	const SUBSET_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'All scripts' },
		{ value: 'latin', label: 'Latin' },
		{ value: 'cyrillic', label: 'Cyrillic' },
		{ value: 'greek', label: 'Greek' },
		{ value: 'arabic', label: 'Arabic' },
		{ value: 'devanagari', label: 'Devanagari' },
		{ value: 'hebrew', label: 'Hebrew' },
		{ value: 'thai', label: 'Thai' },
		{ value: 'vietnamese', label: 'Vietnamese' }
	];
	const TECHNOLOGY_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'Any technology' },
		{ value: 'variable', label: 'Variable', description: 'One or more variable axes' },
		{ value: 'static', label: 'Static', description: 'Individual font files' }
	];
	const AVAILABILITY_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'Any availability' },
		{ value: 'available', label: 'Available', description: 'Not managed by FontNest' },
		{ value: 'managed', label: 'Managed', description: 'Installed through FontNest' }
	];
	const SORT_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'name-asc', label: 'Name A–Z' },
		{ value: 'name-desc', label: 'Name Z–A' },
		{ value: 'recent', label: 'Recently updated' },
		{ value: 'styles', label: 'Most styles' }
	];

	const SAMPLE_FAMILIES: GoogleFontFamilySummary[] = [
		{
			id: 'gf:inter',
			family: 'Inter',
			category: 'sans-serif',
			subsets: ['latin', 'latin-ext', 'cyrillic'],
			license: 'OFL-1.1',
			artifactCount: 2,
			previewArtifactId: 'gf:inter:preview',
			variable: true,
			lastModified: '2026-06-18',
			installed: true
		},
		{
			id: 'gf:jetbrains-mono',
			family: 'JetBrains Mono',
			category: 'monospace',
			subsets: ['latin', 'latin-ext', 'cyrillic'],
			license: 'OFL-1.1',
			artifactCount: 2,
			previewArtifactId: 'gf:jetbrains-mono:preview',
			variable: true,
			lastModified: '2026-05-22',
			installed: false
		},
		{
			id: 'gf:noto-sans',
			family: 'Noto Sans',
			category: 'sans-serif',
			subsets: ['latin', 'greek', 'cyrillic', 'devanagari'],
			license: 'OFL-1.1',
			artifactCount: 4,
			previewArtifactId: 'gf:noto-sans:preview',
			variable: true,
			lastModified: '2026-04-03',
			installed: false
		},
		{
			id: 'gf:playfair-display',
			family: 'Playfair Display',
			category: 'serif',
			subsets: ['latin', 'latin-ext', 'vietnamese'],
			license: 'OFL-1.1',
			artifactCount: 2,
			previewArtifactId: 'gf:playfair-display:preview',
			variable: true,
			lastModified: '2026-03-14',
			installed: false
		},
		{
			id: 'gf:roboto',
			family: 'Roboto',
			category: 'sans-serif',
			subsets: ['latin', 'greek', 'cyrillic'],
			license: 'Apache-2.0',
			artifactCount: 8,
			previewArtifactId: 'gf:roboto:preview',
			variable: false,
			lastModified: '2026-02-11',
			installed: false
		},
		{
			id: 'gf:source-serif-4',
			family: 'Source Serif 4',
			category: 'serif',
			subsets: ['latin', 'latin-ext', 'vietnamese'],
			license: 'OFL-1.1',
			artifactCount: 2,
			previewArtifactId: 'gf:source-serif-4:preview',
			variable: true,
			lastModified: '2026-01-28',
			installed: false
		}
	];

	let {
		installedFamilyNames,
		onInstalled,
		onToast
	}: {
		installedFamilyNames: string[];
		onInstalled: () => void | Promise<void>;
		onToast: (message: string, tone: 'success' | 'error') => void;
	} = $props();

	let nativeMode = $state(false);
	let page = $state<GoogleFontPage | null>(null);
	let details = $state<GoogleFontFamilyDetails | null>(null);
	let selectedFamilyId = $state<string | null>(null);
	let selectedArtifactIds = $state<string[]>([]);
	let search = $state('');
	let category = $state('all');
	let subset = $state('all');
	let technology = $state('all');
	let availability = $state('all');
	let sort = $state('name-asc');
	let specimenMode = $state<SpecimenMode>('names');
	let customSpecimenText = $state(DEFAULT_CUSTOM_TEXT);
	let specimenSize = $state(DEFAULT_SPECIMEN_SIZE);
	let loadingCatalogue = $state(true);
	let loadMoreSentinel = $state<HTMLElement | null>(null);
	let loadingDetails = $state(false);
	let installing = $state(false);
	let confirmingInstall = $state(false);
	let catalogueError = $state('');
	let detailsError = $state('');
	let previewFamilies = $state<Record<string, string>>({});
	let previewStatuses = $state<Record<string, PreviewStatus>>({});
	let discoverScroll = $state<HTMLElement>();
	let discoverControls = $state<HTMLElement>();
	let discoverControlsElevated = $state(false);
	let installDialog = $state<HTMLDialogElement>();
	let searchTimer: ReturnType<typeof setTimeout> | undefined;
	let catalogueRequest = 0;
	let detailsRequest = 0;
	let destroyed = false;

	const previewQueue = new KeyedTaskQueue<LoadedPreview>(PREVIEW_CONCURRENCY);
	const loadedPreviewFaces = new SvelteMap<string, FontFace>();
	const visiblePreviewIds = new SvelteSet<string>();
	let previewUseOrder: string[] = [];

	let families = $derived(page?.families ?? []);
	let selectedFamily = $derived(
		families.find((family) => family.id === selectedFamilyId) ?? null
	);
	let localFamilyPresent = $derived(
		selectedFamily
			? installedFamilyNames.some(
					(name) => name.toLocaleLowerCase() === selectedFamily.family.toLocaleLowerCase()
				)
			: false
	);
	let selectedBytes = $derived(
		(details?.artifacts ?? [])
			.filter((artifact) => selectedArtifactIds.includes(artifact.id))
			.reduce((total, artifact) => total + artifact.sizeBytes, 0)
	);
	let activeFilters = $derived.by(() => {
		const filters: ActiveFilter[] = [];
		if (category !== 'all')
			filters.push({ key: 'category', label: optionLabel(CATEGORY_OPTIONS, category) });
		if (subset !== 'all')
			filters.push({ key: 'subset', label: optionLabel(SUBSET_OPTIONS, subset) });
		if (technology !== 'all')
			filters.push({ key: 'technology', label: optionLabel(TECHNOLOGY_OPTIONS, technology) });
		if (availability !== 'all')
			filters.push({
				key: 'availability',
				label: optionLabel(AVAILABILITY_OPTIONS, availability)
			});
		if (sort !== 'name-asc')
			filters.push({ key: 'sort', label: optionLabel(SORT_OPTIONS, sort) });
		return filters;
	});
	let hasResettableState = $derived(
		Boolean(search) ||
			activeFilters.length > 0 ||
			specimenMode !== 'names' ||
			customSpecimenText !== DEFAULT_CUSTOM_TEXT ||
			specimenSize !== DEFAULT_SPECIMEN_SIZE
	);

	onMount(() => {
		nativeMode = '__TAURI_INTERNALS__' in window;
		void loadCatalogue(true);
	});

	$effect(() => {
		if (!installDialog) return;
		if (confirmingInstall && !installDialog.open) {
			installDialog.showModal();
		} else if (!confirmingInstall && installDialog.open) {
			installDialog.close();
		}
	});

	$effect(() => {
		const sentinel = loadMoreSentinel;
		if (!sentinel) return;
		const observer = new IntersectionObserver(
			(entries) => {
				if (
					entries.some((entry) => entry.isIntersecting) &&
					!loadingCatalogue &&
					page &&
					page.families.length < page.total
				) {
					void loadCatalogue(false);
				}
			},
			{ rootMargin: '400px' }
		);
		observer.observe(sentinel);
		return () => observer.disconnect();
	});

	onDestroy(() => {
		destroyed = true;
		if (searchTimer) clearTimeout(searchTimer);
		for (const face of loadedPreviewFaces.values()) document.fonts.delete(face);
		loadedPreviewFaces.clear();
		visiblePreviewIds.clear();
	});

	async function loadCatalogue(reset: boolean) {
		const requestId = ++catalogueRequest;
		loadingCatalogue = true;
		catalogueError = '';
		if (reset) {
			page = null;
			clearSelection();
		}

		if (!nativeMode) {
			const matching = filterBrowserFamilies();
			const offset = reset ? 0 : (page?.families.length ?? 0);
			const nextFamilies = matching.slice(offset, offset + PAGE_SIZE);
			page = {
				families: reset ? nextFamilies : [...(page?.families ?? []), ...nextFamilies],
				total: matching.length,
				offset,
				limit: PAGE_SIZE,
				snapshot: 'browser fixture'
			};
			loadingCatalogue = false;
			return;
		}

		try {
			const offset = reset ? 0 : (page?.families.length ?? 0);
			const response = await listGoogleFonts({
				query: search.trim(),
				category,
				subset,
				technology,
				availability,
				sort,
				offset,
				limit: PAGE_SIZE
			});
			if (requestId !== catalogueRequest) return;
			page = reset
				? response
				: { ...response, families: [...(page?.families ?? []), ...response.families] };
		} catch (error) {
			if (requestId !== catalogueRequest) return;
			catalogueError = commandErrorMessage(
				error,
				'The Google Fonts catalogue is unavailable.'
			);
		} finally {
			if (requestId === catalogueRequest) loadingCatalogue = false;
		}
	}

	function filterBrowserFamilies() {
		const queryTerms = search
			.toLocaleLowerCase()
			.split(/\s+/)
			.map((term) => term.trim())
			.filter(Boolean);
		const matching = SAMPLE_FAMILIES.filter((family) => {
			const searchable = [family.family, family.category, family.license, ...family.subsets]
				.join(' ')
				.toLocaleLowerCase();
			return (
				queryTerms.every((term) => searchable.includes(term)) &&
				(category === 'all' || family.category === category) &&
				(subset === 'all' || family.subsets.includes(subset)) &&
				(technology === 'all' || family.variable === (technology === 'variable')) &&
				(availability === 'all' || family.installed === (availability === 'managed'))
			);
		});
		return matching.sort((left, right) => {
			if (sort === 'name-desc') return right.family.localeCompare(left.family);
			if (sort === 'recent') return right.lastModified.localeCompare(left.lastModified);
			if (sort === 'styles') return right.artifactCount - left.artifactCount;
			return left.family.localeCompare(right.family);
		});
	}

	function updateSearch(value: string) {
		search = value;
		if (searchTimer) clearTimeout(searchTimer);
		searchTimer = setTimeout(() => void loadCatalogue(true), 220);
	}

	function updateFilter(key: FilterKey, value: string) {
		if (key === 'category') category = value;
		if (key === 'subset') subset = value;
		if (key === 'technology') technology = value;
		if (key === 'availability') availability = value;
		if (key === 'sort') sort = value;
		void loadCatalogue(true);
	}

	function clearFilter(key: FilterKey) {
		updateFilter(key, key === 'sort' ? 'name-asc' : 'all');
	}

	function resetAll() {
		if (searchTimer) clearTimeout(searchTimer);
		search = '';
		category = 'all';
		subset = 'all';
		technology = 'all';
		availability = 'all';
		sort = 'name-asc';
		specimenMode = 'names';
		customSpecimenText = DEFAULT_CUSTOM_TEXT;
		specimenSize = DEFAULT_SPECIMEN_SIZE;
		void loadCatalogue(true);
	}

	async function selectFamily(familyId: string) {
		if (selectedFamilyId === familyId) {
			clearSelection();
			return;
		}
		selectedFamilyId = familyId;
		await loadDetails(familyId);
	}

	function clearSelection() {
		detailsRequest += 1;
		selectedFamilyId = null;
		details = null;
		detailsError = '';
		loadingDetails = false;
		selectedArtifactIds = [];
		confirmingInstall = false;
	}

	async function loadDetails(familyId: string) {
		const requestId = ++detailsRequest;
		loadingDetails = true;
		detailsError = '';
		details = null;

		try {
			const response = nativeMode
				? await getGoogleFontDetails(familyId)
				: sampleDetails(
						SAMPLE_FAMILIES.find((family) => family.id === familyId) ??
							SAMPLE_FAMILIES[0]
					);
			if (requestId !== detailsRequest || selectedFamilyId !== familyId) return;
			details = response;
			selectedArtifactIds = response.artifacts
				.filter((artifact) => !artifact.installed)
				.map((artifact) => artifact.id);
		} catch (error) {
			if (requestId !== detailsRequest || selectedFamilyId !== familyId) return;
			detailsError = commandErrorMessage(error, 'Font details are unavailable.');
		} finally {
			if (requestId === detailsRequest) loadingDetails = false;
		}
	}

	function observePreview(node: HTMLElement, family: GoogleFontFamilySummary) {
		const observer = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					if (entry.isIntersecting) {
						visiblePreviewIds.add(family.id);
						void requestPreview(family);
					} else {
						visiblePreviewIds.delete(family.id);
					}
				}
			},
			{ root: discoverScroll ?? null, rootMargin: '420px 0px', threshold: 0.01 }
		);
		observer.observe(node);

		return {
			destroy() {
				observer.disconnect();
				visiblePreviewIds.delete(family.id);
			}
		};
	}

	async function requestPreview(family: GoogleFontFamilySummary) {
		const status = previewStatuses[family.id] ?? 'idle';
		if (status === 'loading' || status === 'loaded' || status === 'error') {
			if (status === 'loaded') touchPreview(family.id);
			return;
		}

		if (!nativeMode) {
			previewFamilies[family.id] = `"${family.family}", system-ui, sans-serif`;
			previewStatuses[family.id] = 'loaded';
			return;
		}

		previewStatuses[family.id] = 'loading';
		try {
			const loaded = await previewQueue.enqueue(family.id, async () => {
				const preview = await prepareGoogleFontPreview(family.previewArtifactId);
				const face = new FontFace(preview.fontFamily, `url(${preview.dataUrl})`);
				await face.load();
				return { face, fontFamily: preview.fontFamily };
			});
			if (destroyed) return;
			document.fonts.add(loaded.face);
			loadedPreviewFaces.set(family.id, loaded.face);
			previewFamilies[family.id] = `"${loaded.fontFamily}", system-ui, sans-serif`;
			previewStatuses[family.id] = 'loaded';
			touchPreview(family.id);
			enforcePreviewCacheLimit();
		} catch {
			if (destroyed) return;
			previewStatuses[family.id] = 'error';
		}
	}

	function touchPreview(familyId: string) {
		previewUseOrder = [...previewUseOrder.filter((id) => id !== familyId), familyId];
	}

	function enforcePreviewCacheLimit() {
		while (loadedPreviewFaces.size > MAX_LOADED_PREVIEWS) {
			const candidate = pickPreviewEvictionCandidate(
				previewUseOrder,
				visiblePreviewIds,
				selectedFamilyId
			);
			if (!candidate) return;
			const face = loadedPreviewFaces.get(candidate);
			if (face) document.fonts.delete(face);
			loadedPreviewFaces.delete(candidate);
			previewUseOrder = previewUseOrder.filter((id) => id !== candidate);
			delete previewFamilies[candidate];
			previewStatuses[candidate] = 'idle';
		}
	}

	function previewStatus(familyId: string): PreviewStatus {
		return previewStatuses[familyId] ?? 'idle';
	}

	function specimenText(family: GoogleFontFamilySummary) {
		return specimenMode === 'names'
			? family.family
			: customSpecimenText.trim() || family.family;
	}

	function handleDiscoverScroll(event: Event) {
		const scrollContainer = event.currentTarget as HTMLElement;
		const elevated = isStickySurfaceElevated(
			scrollContainer.scrollTop,
			discoverControls?.offsetTop ?? Number.POSITIVE_INFINITY
		);
		if (elevated !== discoverControlsElevated) discoverControlsElevated = elevated;
	}

	function toggleArtifact(artifactId: string) {
		selectedArtifactIds = selectedArtifactIds.includes(artifactId)
			? selectedArtifactIds.filter((id) => id !== artifactId)
			: [...selectedArtifactIds, artifactId];
	}

	async function confirmInstall() {
		if (!details || !selectedArtifactIds.length || !nativeMode) return;
		installing = true;
		try {
			const artifactsToActivate = details.artifacts.filter((artifact) =>
				selectedArtifactIds.includes(artifact.id)
			);
			const result = await installGoogleFont(details.id, selectedArtifactIds);
			const installedIds = new Set([
				...result.installedArtifactIds,
				...result.alreadyInstalledArtifactIds
			]);
			details = {
				...details,
				artifacts: details.artifacts.map((artifact) => ({
					...artifact,
					installed: artifact.installed || installedIds.has(artifact.id)
				}))
			};
			if (page) {
				page = {
					...page,
					families: page.families.map((family) =>
						family.id === result.familyId ? { ...family, installed: true } : family
					)
				};
			}
			let livePreviewReady = true;
			try {
				await activateInstalledGoogleFont(result.familyName, artifactsToActivate);
			} catch {
				livePreviewReady = false;
			}
			selectedArtifactIds = [];
			confirmingInstall = false;
			onToast(
				!livePreviewReady
					? `${result.familyName} was installed, but its live preview could not be activated in this session.`
					: result.installedArtifactIds.length
						? `${result.familyName} was installed for your Windows account.`
						: `${result.familyName} is already managed by FontNest.`,
				livePreviewReady ? 'success' : 'error'
			);
			await onInstalled();
		} catch (error) {
			onToast(commandErrorMessage(error, 'FontNest could not install that font.'), 'error');
		} finally {
			installing = false;
		}
	}

	function sampleDetails(family: GoogleFontFamilySummary): GoogleFontFamilyDetails {
		return {
			id: family.id,
			family: family.family,
			category: family.category,
			subsets: family.subsets,
			license: family.license,
			lastModified: family.lastModified,
			version: 'Browser fixture',
			previewArtifactId: family.previewArtifactId,
			artifacts: [
				{
					id: `${family.id}:regular`,
					fileName: `${family.family.replaceAll(' ', '')}${family.variable ? '[wght]' : '-Regular'}.ttf`,
					style: family.variable ? 'Variable' : 'Regular',
					format: 'TrueType',
					sizeBytes: 480_000,
					installed: family.installed
				},
				{
					id: `${family.id}:italic`,
					fileName: `${family.family.replaceAll(' ', '')}-Italic${family.variable ? '[wght]' : ''}.ttf`,
					style: family.variable ? 'Variable Italic' : 'Italic',
					format: 'TrueType',
					sizeBytes: 510_000,
					installed: family.installed
				}
			]
		};
	}

	function optionLabel(options: DiscoverFilterOption[], value: string) {
		return options.find((option) => option.value === value)?.label ?? value;
	}

	function categoryLabel(value: string) {
		return optionLabel(CATEGORY_OPTIONS, value);
	}

	function commandErrorMessage(error: unknown, fallback: string): string {
		if (typeof error === 'object' && error && 'message' in error) return String(error.message);
		return fallback;
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}
</script>

<section
	bind:this={discoverScroll}
	class="discover-view"
	aria-labelledby="discover-title"
	onscroll={handleDiscoverScroll}
>
	<header class="discover-header">
		<div>
			<h1 id="discover-title">Discover open fonts</h1>
			<p>
				{page?.total.toLocaleString() ?? '—'} families from the trusted Google Fonts catalogue
			</p>
		</div>
		<div class="source-status">
			<span class:native={nativeMode}></span>
			<div>
				<strong>{nativeMode ? 'Secure desktop catalogue' : 'Browser fixture'}</strong>
				<small>{page?.snapshot ?? 'Reading source snapshot'}</small>
			</div>
		</div>
	</header>

	{#if !nativeMode}
		<div class="desktop-notice" role="note">
			<Icon name="alert" size={15} />
			<span>Run <code>pnpm desktop</code> to download real previews or install fonts.</span>
		</div>
	{/if}

	<section
		bind:this={discoverControls}
		class:is-elevated={discoverControlsElevated}
		class="discover-controls sticky-control-surface"
		aria-label="Discover controls"
	>
		<div class="primary-toolbar">
			<label class="search-control">
				<span>Search</span>
				<Icon name="search" size={15} />
				<input
					type="search"
					placeholder="Families, scripts, licences"
					value={search}
					oninput={(event) => updateSearch(event.currentTarget.value)}
				/>
			</label>
			<div class="filter-strip">
				<DiscoverFilterMenu
					id="discover-category"
					label="Category"
					value={category}
					options={CATEGORY_OPTIONS}
					onChange={(value) => updateFilter('category', value)}
				/>
				<DiscoverFilterMenu
					id="discover-script"
					label="Script"
					value={subset}
					options={SUBSET_OPTIONS}
					onChange={(value) => updateFilter('subset', value)}
				/>
				<DiscoverFilterMenu
					id="discover-technology"
					label="Technology"
					value={technology}
					options={TECHNOLOGY_OPTIONS}
					onChange={(value) => updateFilter('technology', value)}
				/>
				<DiscoverFilterMenu
					id="discover-availability"
					label="Availability"
					value={availability}
					options={AVAILABILITY_OPTIONS}
					onChange={(value) => updateFilter('availability', value)}
				/>
				<DiscoverFilterMenu
					id="discover-sort"
					label="Sort"
					value={sort}
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
					value={customSpecimenText}
					placeholder="Type a shared specimen"
					disabled={specimenMode === 'names'}
					oninput={(event) => (customSpecimenText = event.currentTarget.value)}
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
					min="64"
					max="156"
					step="4"
					value={specimenSize}
					oninput={(event) => (specimenSize = Number(event.currentTarget.value))}
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
			<strong>{page?.total.toLocaleString() ?? 0} families</strong>
			<span>Names render in their own typeface as they enter the window</span>
		</div>

		{#if loadingCatalogue && !families.length}
			<div class="specimen-list" aria-label="Loading font families">
				{#each SKELETON_ROWS as row (row)}
					<div class="specimen-entry loading-entry" aria-hidden="true">
						<div class="loading-meta"><span></span><span></span><span></span></div>
						<div class="specimen-skeleton"><span></span><span></span><span></span></div>
					</div>
				{/each}
			</div>
		{:else if catalogueError}
			<div class="catalogue-state" role="alert">
				<div class="state-icon error"><Icon name="alert" size={20} /></div>
				<h2>Online catalogue unavailable</h2>
				<p>{catalogueError}</p>
				<button type="button" onclick={() => void loadCatalogue(true)}>Try again</button>
			</div>
		{:else if !families.length}
			<div class="catalogue-state">
				<div class="state-icon"><Icon name="search" size={20} /></div>
				<h2>No families match</h2>
				<p>Try a broader search or remove one of the active filters.</p>
				<button type="button" onclick={resetAll}>Reset discover</button>
			</div>
		{:else}
			<div class="specimen-list" aria-label="Google Fonts families">
				{#each families as family (family.id)}
					<article class:selected={selectedFamilyId === family.id} class="specimen-entry">
						<button
							type="button"
							class="specimen-toggle"
							aria-expanded={selectedFamilyId === family.id}
							aria-controls={`family-details-${family.id}`}
							onclick={() => void selectFamily(family.id)}
						>
							<span class="family-line">
								<strong>{family.family}</strong>
								<span>{categoryLabel(family.category)}</span>
								<span
									>{family.artifactCount}
									{family.artifactCount === 1 ? 'style' : 'styles'}</span
								>
								<span>{family.variable ? 'Variable' : 'Static'}</span>
								<span>{family.license}</span>
								<span class:managed={family.installed} class="availability-label">
									<span></span>{family.installed ? 'Managed' : 'Available'}
								</span>
								<span class="open-label">
									{selectedFamilyId === family.id
										? 'Close details'
										: 'Open family'}
									<Icon name="chevron" size={13} />
								</span>
							</span>

							<span
								use:observePreview={family}
								class:ready={previewStatus(family.id) === 'loaded'}
								class:error={previewStatus(family.id) === 'error'}
								class="specimen-canvas"
							>
								{#if previewStatus(family.id) === 'loaded'}
									<span
										class="specimen-text"
										style:font-family={previewFamilies[family.id]}
										>{specimenText(family)}</span
									>
								{:else if previewStatus(family.id) === 'error'}
									<span class="specimen-text fallback"
										>{specimenText(family)}</span
									>
									<small>Preview unavailable · system fallback</small>
								{:else}
									<span class="specimen-skeleton" aria-hidden="true">
										<span></span><span></span><span></span>
									</span>
									<small
										>{previewStatus(family.id) === 'loading'
											? 'Loading preview'
											: 'Waiting for viewport'}</small
									>
								{/if}
							</span>
						</button>

						{#if selectedFamilyId === family.id}
							<section
								id={`family-details-${family.id}`}
								class="family-details"
								aria-live="polite"
							>
								{#if loadingDetails}
									<div class="detail-message">Reading family files…</div>
								{:else if detailsError}
									<div class="detail-message error">
										<Icon name="alert" size={15} />
										{detailsError}
									</div>
								{:else if details}
									<div class="detail-heading">
										<div>
											<strong
												>{details.artifacts.length} desktop font files</strong
											>
											<p>{details.subsets.slice(0, 8).join(' · ')}</p>
										</div>
										<div class="detail-facts">
											<span>{details.version}</span>
											<span>Updated {details.lastModified}</span>
											{#if localFamilyPresent && !family.installed}<span
													>Found locally</span
												>{/if}
										</div>
									</div>

									<div class="artifact-list">
										{#each details.artifacts as artifact (artifact.id)}
											<label class:installed={artifact.installed}>
												<input
													type="checkbox"
													checked={artifact.installed ||
														selectedArtifactIds.includes(artifact.id)}
													disabled={artifact.installed || !nativeMode}
													onchange={() => toggleArtifact(artifact.id)}
												/>
												<span>
													<strong>{artifact.style}</strong>
													<small>{artifact.fileName}</small>
												</span>
												<small
													>{artifact.installed
														? 'Installed'
														: formatBytes(artifact.sizeBytes)}</small
												>
											</label>
										{/each}
									</div>

									<footer class="install-footer">
										<div>
											<strong
												>{selectedArtifactIds.length} selected · {formatBytes(
													selectedBytes
												)}</strong
											>
											<small>Per-user install · no administrator access</small
											>
										</div>
										<button
											type="button"
											class="install-action"
											disabled={!nativeMode ||
												!selectedArtifactIds.length ||
												installing}
											onclick={() => (confirmingInstall = true)}
										>
											{details.artifacts.every(
												(artifact) => artifact.installed
											)
												? 'Managed'
												: `Review ${selectedArtifactIds.length} ${selectedArtifactIds.length === 1 ? 'file' : 'files'}`}
										</button>
									</footer>
								{/if}
							</section>
						{/if}
					</article>
				{/each}
			</div>

			{#if page && page.families.length < page.total}
				<div
					class="load-more-sentinel"
					bind:this={loadMoreSentinel}
					aria-hidden="true"
				></div>
				<div class="load-more-row">
					<button
						type="button"
						disabled={loadingCatalogue}
						onclick={() => void loadCatalogue(false)}
					>
						{loadingCatalogue ? 'Loading more families…' : 'Load more families'}
					</button>
					<span
						>{page.families.length.toLocaleString()} of {page.total.toLocaleString()}</span
					>
				</div>
			{/if}
		{/if}
	</div>
</section>

{#if details}
	<dialog
		bind:this={installDialog}
		class="install-dialog"
		aria-labelledby="install-dialog-title"
		onclose={() => (confirmingInstall = false)}
		oncancel={(event) => {
			if (installing) event.preventDefault();
			else confirmingInstall = false;
		}}
		onclick={(event) => {
			if (event.target === installDialog && !installing) confirmingInstall = false;
		}}
	>
		<div class="dialog-icon"><Icon name="font" size={20} /></div>
		<h2 id="install-dialog-title">Install {details.family}?</h2>
		<p>
			FontNest will download and verify {selectedArtifactIds.length}
			{selectedArtifactIds.length === 1 ? 'file' : 'files'}, then register them for your
			Windows account.
		</p>
		<dl>
			<div>
				<dt>Source</dt>
				<dd>Google Fonts</dd>
			</div>
			<div>
				<dt>Licence</dt>
				<dd>{details.license}</dd>
			</div>
			<div>
				<dt>Scope</dt>
				<dd>Current user</dd>
			</div>
			<div>
				<dt>Download</dt>
				<dd>{formatBytes(selectedBytes)}</dd>
			</div>
		</dl>
		<div class="dialog-actions">
			<button type="button" disabled={installing} onclick={() => (confirmingInstall = false)}
				>Cancel</button
			>
			<button
				type="button"
				class="confirm-action"
				disabled={installing}
				onclick={confirmInstall}
			>
				{installing ? 'Installing securely…' : 'Install font files'}
			</button>
		</div>
	</dialog>
{/if}

<style>
	.discover-view {
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

	.discover-header {
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

	.discover-header > div:first-child {
		min-width: 0;
	}

	h1,
	h2,
	p {
		margin-top: 0;
	}

	.discover-header h1 {
		margin-bottom: 4px;
		font-size: var(--text-heading);
		letter-spacing: -0.03em;
	}

	.discover-header p {
		margin-bottom: 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	.source-status {
		display: flex;
		min-width: 210px;
		flex: none;
		align-items: center;
		gap: 9px;
	}

	.source-status > span {
		width: 8px;
		height: 8px;
		flex: none;
		border-radius: 50%;
		background: var(--color-warning);
	}

	.source-status > span.native {
		background: var(--color-success);
	}

	.source-status div {
		display: grid;
		min-width: 0;
		gap: 2px;
	}

	.source-status strong,
	.source-status small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.source-status strong {
		font-size: var(--text-label);
	}

	.source-status small {
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.desktop-notice {
		display: flex;
		flex: none;
		align-items: center;
		gap: 9px;
		padding: 8px 24px;
		border-bottom: 1px solid color-mix(in srgb, var(--color-warning) 38%, var(--color-border));
		color: var(--color-warning);
		background: color-mix(in srgb, var(--color-warning) 7%, transparent);
		font-size: var(--text-label);
	}

	.desktop-notice code {
		font-size: inherit;
	}

	.discover-controls {
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
		contain-intrinsic-size: 250px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface);
		transition: background var(--motion-fast);
	}

	.specimen-entry.selected {
		content-visibility: visible;
		background: color-mix(in srgb, var(--color-selected) 38%, var(--color-surface));
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
		display: grid;
		min-height: 45px;
		grid-template-columns: minmax(150px, 1fr) repeat(4, auto) auto auto;
		align-items: center;
		gap: clamp(10px, 1.35vw, 22px);
		padding: 0 24px;
		color: var(--color-subtle);
		font-family: Geist, 'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif;
		font-size: var(--text-micro);
	}

	.family-line > strong {
		overflow: hidden;
		color: var(--color-text);
		font-size: var(--text-label);
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.availability-label,
	.open-label {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		white-space: nowrap;
	}

	.availability-label > span {
		width: 6px;
		height: 6px;
		border: 1px solid var(--color-subtle);
		border-radius: 50%;
	}

	.availability-label.managed {
		color: var(--color-success);
	}

	.availability-label.managed > span {
		border-color: var(--color-success);
		background: var(--color-success);
	}

	.open-label {
		justify-self: end;
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
		position: relative;
		display: flex;
		min-height: calc(var(--specimen-size) * 1.2 + 56px);
		align-items: center;
		padding: 22px 64px 28px;
		overflow: hidden;
	}

	.specimen-canvas > small {
		position: absolute;
		right: 24px;
		bottom: 12px;
		color: var(--color-subtle);
		font-family: Geist, 'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif;
		font-size: var(--text-micro);
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

	.specimen-text.fallback {
		font-family: Geist, 'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif;
		color: var(--color-muted);
	}

	.specimen-skeleton {
		display: flex;
		width: min(900px, 82%);
		align-items: center;
		gap: 10px;
	}

	.specimen-skeleton span {
		height: clamp(58px, 7vw, 92px);
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

	.loading-entry > .specimen-skeleton {
		min-height: 170px;
		margin-left: 64px;
	}

	.family-details {
		padding: 18px 24px 20px;
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

	.detail-facts {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 6px 14px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.detail-message {
		display: flex;
		min-height: 92px;
		align-items: center;
		justify-content: center;
		gap: var(--space-sm);
		color: var(--color-muted);
		font-size: var(--text-label);
	}

	.detail-message.error {
		color: var(--color-danger);
	}

	.artifact-list {
		display: grid;
		max-height: 230px;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		margin-top: 14px;
		overflow-y: auto;
		border-top: 1px solid var(--color-border);
	}

	.artifact-list label {
		display: grid;
		min-height: 52px;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 10px;
		padding-right: 14px;
		border-bottom: 1px solid var(--color-border);
		font-size: var(--text-label);
		cursor: pointer;
	}

	.artifact-list label:nth-child(odd) {
		padding-right: 18px;
		border-right: 1px solid var(--color-border);
	}

	.artifact-list label:nth-child(even) {
		padding-left: 18px;
	}

	.artifact-list label.installed {
		color: var(--color-muted);
		cursor: default;
	}

	.artifact-list input {
		width: 16px;
		height: 16px;
		accent-color: var(--color-accent);
	}

	.artifact-list label > span {
		display: grid;
		min-width: 0;
		gap: 2px;
	}

	.artifact-list small {
		overflow: hidden;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.install-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-xl);
		padding-top: 16px;
	}

	.install-footer > div {
		display: grid;
		gap: 3px;
		font-size: var(--text-label);
	}

	.install-footer small {
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.install-action,
	.load-more-row button,
	.catalogue-state button,
	.dialog-actions button {
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

	.install-action {
		border-color: var(--color-accent);
		color: var(--color-accent-ink);
		background: var(--color-accent);
	}

	.install-action:disabled,
	.load-more-row button:disabled {
		opacity: 0.5;
	}

	.load-more-sentinel {
		height: 1px;
		pointer-events: none;
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

	.install-dialog {
		position: fixed;
		top: 50%;
		left: 50%;
		z-index: var(--z-modal);
		width: min(460px, calc(100vw - 32px));
		max-height: calc(100dvh - 32px);
		margin: 0;
		padding: 24px;
		overflow-y: auto;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-text);
		background: var(--color-raised);
		box-shadow: var(--shadow-floating);
		transform: translate(-50%, -50%);
		overscroll-behavior: contain;
	}

	.install-dialog::backdrop {
		background: color-mix(in srgb, var(--color-bg) 72%, transparent);
		backdrop-filter: blur(5px);
	}

	.dialog-icon {
		display: grid;
		width: 40px;
		height: 40px;
		place-items: center;
		margin-bottom: 16px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-panel);
	}

	.install-dialog h2 {
		margin-bottom: 7px;
		font-size: var(--text-heading-sm);
	}

	.install-dialog > p {
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.5;
	}

	.install-dialog dl {
		display: grid;
		gap: var(--space-sm);
		margin: 18px 0 0;
		padding: 14px 0;
		border-top: 1px solid var(--color-border);
		border-bottom: 1px solid var(--color-border);
	}

	.install-dialog dl div {
		display: flex;
		justify-content: space-between;
		gap: var(--space-xl);
		font-size: var(--text-body-sm);
	}

	.install-dialog dt {
		color: var(--color-muted);
	}

	.install-dialog dd {
		margin: 0;
		font-weight: 650;
	}

	.dialog-actions {
		display: flex;
		justify-content: flex-end;
		gap: 9px;
		margin-top: 20px;
	}

	.dialog-actions .confirm-action {
		border-color: var(--color-accent);
		color: var(--color-accent-ink);
		background: var(--color-accent);
	}

	@keyframes skeleton-pulse {
		to {
			opacity: 0.52;
		}
	}

	@container (max-width: 980px) {
		.family-line {
			grid-template-columns: minmax(140px, 1fr) repeat(3, auto) auto auto;
		}

		.family-line > span:nth-of-type(4) {
			display: none;
		}

		.specimen-text {
			font-size: min(var(--specimen-size), 12cqi);
		}
	}

	@media (max-width: 1240px) {
		.primary-toolbar {
			grid-template-columns: 1fr;
		}

		.filter-strip {
			justify-content: flex-start;
			overflow-x: auto;
		}

		.filter-strip :global(.filter-control) {
			min-width: 142px;
			flex: 0 0 auto;
		}

		.preview-text-control {
			width: min(310px, 32vw);
		}

		.active-filter-summary {
			display: none;
		}
	}

	@media (max-width: 900px) {
		.discover-header {
			padding-inline: 18px;
		}

		.primary-toolbar,
		.specimen-toolbar {
			padding-inline: 18px;
		}

		.specimen-toolbar {
			flex-wrap: wrap;
		}

		.preview-text-control {
			width: auto;
			flex: 1 1 260px;
		}

		.family-line {
			grid-template-columns: minmax(140px, 1fr) auto auto auto;
			padding-inline: 18px;
		}

		.family-line > span:nth-of-type(2),
		.family-line > span:nth-of-type(3),
		.family-line > span:nth-of-type(4) {
			display: none;
		}

		.specimen-canvas {
			min-height: 160px;
			padding-inline: 38px;
		}

		.artifact-list {
			grid-template-columns: 1fr;
		}

		.artifact-list label:nth-child(n) {
			padding-inline: 0;
			border-right: 0;
		}
	}

	@media (max-width: 620px) {
		.discover-header {
			align-items: flex-start;
			flex-direction: column;
			gap: var(--space-md);
		}

		.source-status {
			width: 100%;
		}

		.primary-toolbar,
		.specimen-toolbar {
			padding-inline: 12px;
		}

		.filter-strip {
			flex-wrap: wrap;
			overflow: visible;
		}

		.filter-strip :global(.filter-control) {
			min-width: min(142px, 100%);
			flex: 1 1 142px;
		}

		.size-control {
			grid-template-columns: auto minmax(80px, 1fr) 44px;
			width: 100%;
		}

		.catalogue-heading {
			padding-inline: 14px;
		}

		.catalogue-heading > span {
			display: none;
		}

		.family-line {
			grid-template-columns: minmax(110px, 1fr) auto;
			padding-inline: 14px;
		}

		.family-line > span:not(.open-label) {
			display: none;
		}

		.open-label {
			font-size: 0;
		}

		.specimen-canvas {
			min-height: 138px;
			padding: 18px 22px 24px;
		}

		.specimen-text {
			font-size: min(var(--specimen-size), 15cqi);
		}

		.family-details {
			padding-inline: 14px;
		}

		.detail-heading,
		.install-footer {
			align-items: stretch;
			flex-direction: column;
		}

		.detail-facts {
			justify-content: flex-start;
		}

		.install-action {
			width: 100%;
		}
	}
</style>
