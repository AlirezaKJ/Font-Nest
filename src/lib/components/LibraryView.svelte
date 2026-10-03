<script lang="ts">
	import { slide } from 'svelte/transition';
	import { quintOut } from 'svelte/easing';

	import { type LibraryFilterKey, activeFiltersFrom, formatValues } from '$lib/library/filters';
	import { PAGE_SIZE, type LibrarySettings } from '$lib/library/settings.svelte';
	import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
	import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
	import type { ContextMenuRequest } from '$lib/context-menu/types';
	import DiscoverFilterMenu, {
		type DiscoverFilterOption
	} from '$lib/components/DiscoverFilterMenu.svelte';
	import FilterPopover, { type FilterGroup } from '$lib/components/FilterPopover.svelte';
	import Icon from './Icon.svelte';
	import RangeSlider from './RangeSlider.svelte';
	import { contextMenu } from '$lib/context-menu/action';
	import {
		FONT_ORIGIN_ORDER,
		familyOrigin,
		fontOrigin,
		isSystemOnly
	} from '$lib/fonts/font-origin';
	import {
		faceDetail,
		faceSpecimenStyle,
		familyPreviewStyle as familyPreviewStyleFor,
		specimenText as specimenTextFor,
		substituteWeightNote as substituteWeightNoteFor,
		weightRange
	} from '$lib/library/specimen';
	import { isStickySurfaceElevated } from '$lib/sticky-surface';
	import { weightName } from '$lib/fonts/weights';

	let {
		library,
		catalogue,
		catalogueMode,
		loading,
		errorMessage,
		prefersReducedMotion,
		density,
		previewText,
		pinnedFamilyIds,
		selectedFamilyId,
		filteredFamilies,
		libraryScrollElement = $bindable(),
		onSetPreviewText,
		onRefreshCatalogue,
		onImport,
		onPreviewFile,
		onOpenFamilyPreview,
		onToggleFamily,
		onTogglePinned,
		onReviewConflict,
		familyMenu,
		faceMenu
	}: {
		/** How the library is being looked at: searched, filtered, ordered, drawn. */
		library: LibrarySettings;
		catalogue: FontCatalogue | null;
		catalogueMode: string;
		loading: boolean;
		errorMessage: string | null;
		prefersReducedMotion: boolean;
		density: string;
		previewText: string;
		pinnedFamilyIds: string[];
		selectedFamilyId: string | null;
		filteredFamilies: FontFamilySummary[];
		/** Held by the route too, which saves and restores where the list was scrolled to. */
		libraryScrollElement: HTMLElement | undefined;
		onSetPreviewText: (value: string) => void;
		onRefreshCatalogue: () => void;
		onImport: () => void;
		onPreviewFile: () => void;
		onOpenFamilyPreview: (familyId: string) => void;
		onToggleFamily: (familyId: string) => void;
		onTogglePinned: (familyId: string) => void;
		onReviewConflict: (familyId: string) => void;
		familyMenu: (family: FontFamilySummary) => ContextMenuRequest;
		faceMenu: (
			family: FontFamilySummary,
			face: FontFamilySummary['faces'][number]
		) => ContextMenuRequest;
	} = $props();

	/** How many faces a family lists before it stops and says how many more there are. */
	const MAX_DETAIL_FACES = 12;
	const SPACING_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'Any spacing' },
		{ value: 'proportional', label: 'Proportional' },
		{ value: 'monospaced', label: 'Monospaced' }
	];

	const TECHNOLOGY_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'Any technology' },
		{
			value: 'variable',
			label: 'Variable',
			description: 'One file covers a range of weights or widths'
		},
		{ value: 'static', label: 'Static', description: 'One file per style' }
	];

	const STATUS_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'Any status' },
		{ value: 'conflict', label: 'Conflicts only', description: 'Families with duplicate files' }
	];

	let originOptions = $derived.by<DiscoverFilterOption[]>(() => {
		const present = new Set(catalogue?.families.flatMap((family) => family.origins) ?? []);
		return [
			{ value: 'all', label: 'Anywhere' },
			...FONT_ORIGIN_ORDER.filter((origin) => present.has(origin)).map((origin) => ({
				value: origin,
				label: fontOrigin(origin).label,
				description: fontOrigin(origin).description
			}))
		];
	});

	let formatOptions = $derived.by<DiscoverFilterOption[]>(() => [
		{ value: 'all', label: 'All formats' },
		...formatValues(catalogue?.families ?? []).map((value) => ({ value, label: value }))
	]);

	let filterGroups = $derived.by<FilterGroup[]>(() => [
		{ key: 'origin', label: 'Origin', value: library.origin, options: originOptions },
		{ key: 'format', label: 'Format', value: library.format, options: formatOptions },
		{
			key: 'technology',
			label: 'Technology',
			value: library.technology,
			options: TECHNOLOGY_OPTIONS
		},
		{ key: 'spacing', label: 'Spacing', value: library.spacing, options: SPACING_OPTIONS },
		{ key: 'status', label: 'Status', value: library.status, options: STATUS_OPTIONS }
	]);

	const SORT_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'name-asc', label: 'Name A–Z' },
		{ value: 'name-desc', label: 'Name Z–A' },
		{ value: 'styles', label: 'Most styles' },
		{ value: 'faces', label: 'Most files' }
	];
	const SKELETON_ROWS = [0, 1, 2, 3];
	const WEIGHT_NOTE_LINGER_MS = 2600;

	// These belong to the view and go with it: the list unmounts when another view is shown, which
	// is also what puts the sticky header back to rest.
	let libraryControlsElement = $state<HTMLElement>();
	let libraryControlsElevated = $state(false);
	let weightNotesVisible = $state(false);
	let weightNoteTimer: ReturnType<typeof setTimeout> | undefined;

	let activeFilters = $derived(activeFiltersFrom(filterGroups));
	let renderedFamilies = $derived(filteredFamilies.slice(0, library.displayLimit));

	function familyPreviewStyle(family: FontFamilySummary): string {
		return familyPreviewStyleFor(family, library.specimenWeight);
	}

	function substituteWeightNote(family: FontFamilySummary): string | null {
		return substituteWeightNoteFor(family, library.specimenWeight);
	}

	function specimenText(family: FontFamilySummary): string {
		return specimenTextFor(
			family,
			library.specimenMode === 'names' ? 'names' : 'custom',
			previewText
		);
	}

	/**
	 * The note answers a question somebody just asked by moving the slider, so it shows itself then
	 * gets out of the way rather than sitting on every row that cannot follow the weight.
	 */
	function showWeightNotes() {
		weightNotesVisible = true;
		if (weightNoteTimer) clearTimeout(weightNoteTimer);
		weightNoteTimer = setTimeout(() => (weightNotesVisible = false), WEIGHT_NOTE_LINGER_MS);
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
</script>

<section
	class="library-view"
	class:compact={density === 'compact'}
	aria-labelledby="library-title"
	bind:this={libraryScrollElement}
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
			<button type="button" class="secondary-action" onclick={onImport}>
				<Icon name="plus" size={16} />
				<span>Import fonts</span>
			</button>
			<button type="button" class="primary-action" onclick={onPreviewFile}>
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
				<span class="sr-only">Search</span>
				<Icon name="search" size={15} />
				<input
					data-font-library.search
					type="search"
					placeholder="Families, styles, origins"
					value={library.search}
					oninput={(event) => library.setSearch(event.currentTarget.value)}
				/>
			</label>
			<div class="filter-strip">
				<FilterPopover
					id="library-filters"
					groups={filterGroups}
					onChange={(key, value) => library.setFilter(key as LibraryFilterKey, value)}
					onClear={() => library.clearFilters()}
				/>
				<DiscoverFilterMenu
					id="library-sort"
					label="Sort"
					value={library.sortOrder}
					options={SORT_OPTIONS}
					onChange={(value) => library.setFilter('sort', value)}
				/>
				<div
					class:empty={activeFilters.length === 0}
					class="active-filter-summary"
					aria-live="polite"
				>
					{#each activeFilters as filter (filter.key)}
						<button
							type="button"
							aria-label={`Remove ${filter.label} filter`}
							onclick={() => library.clearFilter(filter.key)}
						>
							{filter.label}<Icon name="close" size={12} />
						</button>
					{/each}
				</div>
				<button
					type="button"
					class="reset-action"
					disabled={!library.isDirty}
					onclick={() => library.reset()}>Reset all</button
				>
			</div>
		</div>

		<div class="specimen-toolbar">
			<label class="preview-text-control">
				<span class="sr-only">Preview text</span>
				<Icon name="font" size={15} />
				<input
					type="text"
					value={previewText}
					placeholder="Type a shared specimen"
					disabled={library.specimenMode === 'names'}
					oninput={(event) => onSetPreviewText(event.currentTarget.value)}
				/>
			</label>
			<div class="specimen-modes" role="group" aria-label="Specimen text mode">
				<button
					type="button"
					class:active={library.specimenMode === 'names'}
					aria-pressed={library.specimenMode === 'names'}
					onclick={() => (library.specimenMode = 'names')}>Names</button
				>
				<button
					type="button"
					class:active={library.specimenMode === 'custom'}
					aria-pressed={library.specimenMode === 'custom'}
					onclick={() => (library.specimenMode = 'custom')}>Your text</button
				>
			</div>
			<RangeSlider
				label="Size"
				value={library.specimenSize}
				min={48}
				max={148}
				step={4}
				display={`${library.specimenSize}px`}
				onChange={(value) => (library.specimenSize = value)}
			/>
			<RangeSlider
				label="Weight"
				value={library.specimenWeight}
				min={100}
				max={900}
				step={100}
				display={weightName(library.specimenWeight)}
				valueText={`${weightName(library.specimenWeight)} ${library.specimenWeight}`}
				valueWidth="68px"
				onChange={(value) => {
					library.specimenWeight = value;
					showWeightNotes();
				}}
			/>
		</div>
	</section>

	<div class="specimen-feed" style={`--specimen-size: ${library.specimenSize}px`}>
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
				<button type="button" onclick={() => void onRefreshCatalogue()}>Scan again</button>
			</div>
		{:else if !catalogue?.familyCount}
			<div class="catalogue-state">
				<div class="state-icon"><Icon name="font" size={20} /></div>
				<h2 class="type-display">No installed fonts found</h2>
				<p>Scan again, or open a font file to preview it without installing anything.</p>
				<button type="button" onclick={onPreviewFile}>Preview a font file</button>
			</div>
		{:else if !renderedFamilies.length}
			<div class="catalogue-state">
				<div class="state-icon"><Icon name="search" size={20} /></div>
				<h2>No families match</h2>
				<p>Try a shorter library.search, or remove one of the active filters.</p>
				<button
					type="button"
					onclick={() => {
						library.setSearch('');
						library.clearFilters();
					}}>Clear library.search and filters</button
				>
			</div>
		{:else}
			<div class="specimen-list" aria-label="Font families">
				{#each renderedFamilies as family, index (family.id)}
					{@const saved = pinnedFamilyIds.includes(family.id)}
					<article
						use:contextMenu={() => familyMenu(family)}
						class:selected={selectedFamilyId === family.id}
						class="specimen-entry"
					>
						<button
							type="button"
							class="specimen-toggle"
							aria-expanded={selectedFamilyId === family.id}
							aria-controls={`family-details-${family.id}`}
							onclick={() => onToggleFamily(family.id)}
							onkeydown={(event) => handleRowKeydown(event, index)}
						>
							<span class="family-line">
								<strong>{family.name}</strong>
								<span
									class="meta-origin"
									class:is-added={!isSystemOnly(family.origins)}
									title={familyOrigin(family.origins).description}
									>{familyOrigin(family.origins).label}</span
								>
								<span class="meta-format">{family.formats.join(' · ')}</span>
								<span class="meta-count">
									{family.faceCount}
									{family.faceCount === 1 ? 'style' : 'styles'}
								</span>
								<span class="meta-spacing"
									>{family.monospaced ? 'Monospaced' : 'Proportional'}</span
								>
								{#if family.variable}
									<span
										class="meta-variable"
										title="One file covers a range of weights or widths."
										>Variable</span
									>
								{/if}
								{#if family.hasConflict}
									<span class="conflict-label"
										><Icon name="alert" size={12} /> Conflict</span
									>
								{/if}
								<span class="open-label">
									{selectedFamilyId === family.id ? 'Close' : 'Open family'}
									<Icon name="chevron" size={13} />
								</span>
							</span>
							<span class="specimen-canvas" style={familyPreviewStyle(family)}>
								<span class="specimen-text">{specimenText(family)}</span>
								{#if substituteWeightNote(family)}
									<small class:visible={weightNotesVisible}
										>{substituteWeightNote(family)}</small
									>
								{/if}
							</span>
						</button>

						<!-- Outside the disclosure button on purpose: saving a family and
								     opening it are different intents, and a button inside a button
								     is invalid markup. -->
						<button
							type="button"
							class:saved
							class="row-save"
							aria-pressed={saved}
							aria-label={saved
								? `Remove ${family.name} from saved previews`
								: `Save ${family.name} to previews`}
							title={saved ? 'Saved to previews' : 'Save to previews'}
							onclick={() => onTogglePinned(family.id)}
						>
							<Icon name={saved ? 'check' : 'plus'} size={15} />
						</button>

						{#if selectedFamilyId === family.id}
							<section
								id={`family-details-${family.id}`}
								class="family-details"
								in:slide={{
									duration: prefersReducedMotion ? 0 : 240,
									easing: quintOut
								}}
								out:slide={{
									duration: prefersReducedMotion ? 0 : 170,
									easing: quintOut
								}}
							>
								<div class="detail-bar">
									<p class="detail-facts">
										{#if weightRange(family)}
											<span>{weightRange(family)}</span>
										{/if}
										<span
											>{family.fileCount}
											{family.fileCount === 1 ? 'file' : 'files'}</span
										>
										<span>{familyOrigin(family.origins).description}</span>
									</p>
									<div class="detail-actions">
										{#if family.hasConflict}
											<button
												type="button"
												class="detail-action ghost"
												onclick={() => onReviewConflict(family.id)}
											>
												<Icon name="alert" size={15} /> Review conflict
											</button>
										{/if}
										<button
											type="button"
											class="detail-action"
											onclick={() => onOpenFamilyPreview(family.id)}
										>
											Open preview
											<Icon name="chevron" size={14} />
										</button>
									</div>
								</div>

								<!-- Every face draws the same string so the cuts stack and compare;
										     setting each one in its own style name compared different words. -->
								<ul class="face-list">
									{#each family.faces.slice(0, MAX_DETAIL_FACES) as face (face.id)}
										<li use:contextMenu={() => faceMenu(family, face)}>
											<span class="face-meta">
												<span class="face-name">
													<strong>{face.styleName}</strong>
													<span class="face-weight">{face.weight}</span>
												</span>
												<small title={face.fileName}
													>{faceDetail(family, face)}</small
												>
											</span>
											<span
												class="face-specimen"
												style={faceSpecimenStyle(
													family,
													face.weight,
													face.style
												)}>{specimenText(family)}</span
											>
										</li>
									{/each}
								</ul>
								{#if family.faces.length > MAX_DETAIL_FACES}
									<button
										type="button"
										class="more-faces"
										onclick={() => onOpenFamilyPreview(family.id)}
									>
										Show all {family.faceCount} styles in preview
									</button>
								{/if}
							</section>
						{/if}
					</article>
				{/each}
			</div>

			{#if renderedFamilies.length < filteredFamilies.length}
				<div class="load-more-row">
					<button type="button" onclick={() => library.showMore(filteredFamilies.length)}>
						{Math.min(PAGE_SIZE, filteredFamilies.length - renderedFamilies.length)} more
					</button>
					<span
						>{renderedFamilies.length.toLocaleString()} of {filteredFamilies.length.toLocaleString()}</span
					>
				</div>
			{/if}
		{/if}
	</div>
</section>

<style>
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

	/* Importing is the quieter of the two: previewing a file changes nothing, and installing one
	   should not be the louder invitation of the pair. */
	.secondary-action {
		display: inline-flex;
		height: 36px;
		align-items: center;
		justify-content: center;
		gap: 7px;
		padding: 0 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
		transition:
			background var(--motion-fast),
			transform var(--motion-fast);
	}

	.secondary-action:hover {
		background: var(--color-hover);
	}

	.secondary-action:active {
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
		display: flex;
		width: 100%;
		min-width: 0;
		align-items: center;
		gap: var(--space-md);
		padding: 10px 24px;
		border-bottom: 1px solid var(--color-border);
	}

	.search-control,
	.preview-text-control {
		display: grid;
		min-width: 0;
		grid-template-columns: auto minmax(0, 1fr);
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
		border-color: var(--focus-ring-border);
	}

	.search-control:has(input:focus-visible),
	.preview-text-control:has(input:focus-visible) {
		outline: 2px solid var(--focus-ring);
		outline-offset: 2px;
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

	.search-control {
		flex: 0 1 380px;
	}

	.filter-strip {
		display: flex;
		min-width: 0;
		flex: 1 1 auto;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-sm);
	}

	.filter-strip :global(.filter-control) {
		min-width: 0;
		flex: none;
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

	/* The chips take their own line rather than shrink into illegibility: they are the
	   only place an active filter is named now that the trigger just shows a count. */
	.active-filter-summary {
		display: flex;
		min-width: 0;
		flex: 1 1 260px;
		align-items: center;
		gap: 6px;
		overflow-x: auto;
		scrollbar-width: none;
	}

	/* With no chips the region still has to exist for its live announcements, but it must
	   not claim a 260px basis and push Reset all onto a line of its own. */
	.active-filter-summary.empty {
		flex: 0 0 0;
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
		margin-left: auto;
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
		position: relative;
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
		/* Right padding clears the save button, which sits over the row rather than
		   inside the disclosure button. */
		padding: 0 62px 0 24px;
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

	.meta-origin {
		color: var(--color-subtle);
	}

	/* Most libraries are mostly system fonts, so the fonts somebody installed carry the
	   emphasis and stay findable while scrolling. */
	.meta-origin.is-added {
		color: var(--color-text);
		font-weight: 650;
	}

	.meta-variable {
		color: var(--color-text);
		font-weight: 650;
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

	/* Quiet on a list of thousands, but always present: a control that appears only on
	   hover is unreachable by touch and easy to miss by keyboard. */
	.row-save {
		position: absolute;
		top: 7px;
		right: 18px;
		display: inline-flex;
		width: 31px;
		height: 31px;
		align-items: center;
		justify-content: center;
		padding: 0;
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		color: var(--color-subtle);
		background: transparent;
		cursor: pointer;
		transition:
			color var(--motion-fast),
			border-color var(--motion-fast),
			background var(--motion-fast);
	}

	.row-save:hover {
		border-color: var(--color-border);
		color: var(--color-text);
		background: var(--color-control);
	}

	.row-save.saved {
		border-color: color-mix(in srgb, var(--color-accent) 55%, var(--color-border));
		color: var(--color-text);
		background: color-mix(in srgb, var(--color-accent) 12%, var(--color-control));
	}

	.specimen-canvas {
		position: relative;
		display: flex;
		min-height: calc(var(--specimen-size) * 1.42 + 16px);
		align-items: center;
		padding: 14px 24px;
		overflow: hidden;
	}

	/* The canvas carries the family's own font so the specimen inherits it; the note has to
	   opt back out or it would be set in the typeface it is describing. */
	.specimen-canvas > small {
		position: absolute;
		right: 24px;
		bottom: 10px;
		color: var(--color-subtle);
		font-family: Geist, 'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif;
		font-size: var(--text-micro);
		font-weight: 400;
		opacity: 0;
		transition: opacity 420ms cubic-bezier(0.16, 1, 0.3, 1);
		pointer-events: none;
	}

	.specimen-canvas > small.visible {
		opacity: 1;
	}

	@media (prefers-reduced-motion: reduce) {
		.specimen-canvas > small {
			transition: none;
		}

		.specimen-entry,
		.row-save,
		.detail-action,
		.more-faces {
			transition: none;
		}
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

	/* The collapsed row already carries origin, format, style count, spacing, and
	   technology, so this bar only adds what it cannot fit. */
	.detail-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-xl);
	}

	.detail-facts {
		display: flex;
		min-width: 0;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 4px 10px;
		margin: 0;
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.detail-facts span + span::before {
		margin-right: 10px;
		color: var(--color-subtle);
		content: '·';
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

	.face-list {
		display: grid;
		margin: 12px 0 0;
		padding: 0;
		list-style: none;
		border-top: 1px solid var(--color-border);
	}

	.face-list li {
		display: grid;
		min-height: 44px;
		grid-template-columns: minmax(126px, 180px) minmax(0, 1fr);
		align-items: center;
		gap: 20px;
		padding: 7px 0;
		overflow: hidden;
		border-bottom: 1px solid var(--color-border);
	}

	.face-meta {
		display: grid;
		min-width: 0;
		gap: 1px;
	}

	.face-name {
		display: flex;
		align-items: baseline;
		gap: 7px;
	}

	.face-name strong {
		overflow: hidden;
		font-size: var(--text-body-sm);
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.face-weight {
		flex: none;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
	}

	.face-meta small {
		overflow: hidden;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* Smaller than the row specimen above it: this column is a waterfall for comparing
	   cuts, not a second headline. */
	.face-specimen {
		overflow: hidden;
		font-size: clamp(20px, 2.4vw, 32px);
		line-height: 1.25;
		letter-spacing: -0.02em;
		white-space: nowrap;
	}

	.more-faces {
		margin: 12px 0 0;
		padding: 0;
		border: 0;
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-micro);
		font-weight: 650;
		text-decoration: underline;
		text-underline-offset: 3px;
		cursor: pointer;
		transition: color var(--motion-fast);
	}

	.more-faces:hover {
		color: var(--color-text);
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

	/* The display-set state brings its own size from .type-display; the rest stay small. */
	.catalogue-state h2:not(.type-display) {
		margin-bottom: 0;
		font-size: var(--text-heading-sm);
	}

	.catalogue-state h2.type-display {
		margin-bottom: 0;
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

	@keyframes skeleton-pulse {
		50% {
			opacity: 0.45;
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
		.meta-origin,
		.meta-variable,
		.meta-count {
			display: none;
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
			flex-wrap: wrap;
			gap: var(--space-sm);
		}

		.search-control {
			flex: 1 1 100%;
		}

		.filter-strip {
			width: 100%;
		}

		.specimen-toolbar {
			flex-wrap: wrap;
		}

		.preview-text-control {
			width: auto;
			flex: 1 1 240px;
		}

		.catalogue-heading,
		.family-line,
		.specimen-canvas,
		.family-details {
			padding-inline: 16px;
		}

		.family-line {
			padding-right: 54px;
		}

		.row-save {
			right: 12px;
		}

		.detail-bar {
			align-items: stretch;
			flex-direction: column;
			gap: var(--space-md);
		}

		.face-list li {
			grid-template-columns: 1fr;
			gap: 4px;
		}
	}

	@media (max-width: 520px) {
		.catalogue-summary {
			max-width: 32ch;
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
