<script lang="ts" module>
	const WEIGHT_NAMES: Record<number, string> = {
		100: 'Thin',
		200: 'Extralight',
		300: 'Light',
		400: 'Regular',
		500: 'Medium',
		600: 'Semibold',
		700: 'Bold',
		800: 'Extrabold',
		900: 'Black'
	};

	function weightName(weight: number): string {
		const nearest = Object.keys(WEIGHT_NAMES)
			.map(Number)
			.reduce((closest, value) =>
				Math.abs(value - weight) < Math.abs(closest - weight) ? value : closest
			);
		return WEIGHT_NAMES[nearest] ?? 'Regular';
	}

	const INITIAL_GLYPH_BATCH = 128;
	const GLYPH_BATCH_SIZE = 256;
	const GLYPH_VIEW_MODES = [
		{ value: 'fill', label: 'Fill' },
		{ value: 'outline', label: 'Outline' },
		{ value: 'points', label: 'Points' }
	] as const;
	const GLYPH_SET_SCOPES = [
		{ value: 'basic', label: 'Basic' },
		{ value: 'full', label: 'Full' }
	] as const;

	type Alignment = 'left' | 'center';
	type LetterCase = 'as-typed' | 'upper' | 'lower';
	type GlyphViewMode = 'fill' | 'outline' | 'points';
</script>

<script lang="ts">
	import { quintOut } from 'svelte/easing';
	import { slide } from 'svelte/transition';

	import type { FontFaceInspection } from '$lib/bindings/FontFaceInspection';
	import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
	import type { FontGlyphOutline } from '$lib/bindings/FontGlyphOutline';
	import type { FontGlyphVariationValue } from '$lib/bindings/FontGlyphVariationValue';
	import type { FontParserJsonExport } from '$lib/bindings/FontParserJsonExport';
	import {
		filterGlyphSetCodepoints,
		formatCodepoint,
		glyphCellText,
		groupUnicodeCodepoints,
		usesCodepointPlaceholder,
		type GlyphSetScope
	} from '$lib/glyph-categories';
	import {
		exportFontFaceParserJson,
		inspectFontFace,
		inspectFontGlyphOutline
	} from '$lib/tauri/commands';

	import Icon from './Icon.svelte';

	let {
		family,
		previewText,
		previewSize,
		previewWeight,
		pinned,
		onBack,
		onTogglePinned,
		onPreviewText,
		onPreviewSize,
		onPreviewWeight
	}: {
		family: FontFamilySummary | null;
		previewText: string;
		previewSize: number;
		previewWeight: number;
		pinned: boolean;
		onBack: () => void;
		onTogglePinned: () => void;
		onPreviewText: (value: string) => void;
		onPreviewSize: (value: number) => void;
		onPreviewWeight: (value: number) => void;
	} = $props();

	// Local presentation controls — text/size/weight persist via the parent,
	// these shape only how the tester renders and don't need to leave the page.
	let alignment = $state<Alignment>('left');
	let letterCase = $state<LetterCase>('as-typed');
	let stylesSize = $state(46);
	// Glyph weight trails the tester weight until the user picks one explicitly.
	let glyphWeightOverride = $state<number | null>(null);
	let selectedFaceOverrideId = $state<string | null>(null);
	let selectedGlyph = $state('A');
	let lockedGlyph = $state<string | null>(null);
	let glyphSetScope = $state<GlyphSetScope>('full');
	let glyphViewMode = $state<GlyphViewMode>('fill');
	let glyphOutline = $state<FontGlyphOutline | null>(null);
	let glyphOutlineLoading = $state(false);
	let glyphOutlineError = $state('');
	let expandedGlyphCategories = $state<Set<string>>(new Set());
	let glyphCategoryLimits = $state<Record<string, number>>({});
	let glyphCoverageFaceId = $state<string | null>(null);
	let prefersReducedMotion = $state(false);
	let faceInspection = $state<FontFaceInspection | null>(null);
	let inspectionLoading = $state(false);
	let inspectionError = $state('');
	let parserExport = $state<FontParserJsonExport | null>(null);
	let parserLoading = $state(false);
	let parserError = $state('');
	let parserCopyLabel = $state('Copy JSON');
	const glyphOutlineCache = new Map<string, FontGlyphOutline>();

	let specimenEl = $state<HTMLElement>();
	let sentinelEl = $state<HTMLElement>();
	let stuck = $state(false);

	function safeFontStack(name: string): string {
		return `"${name.replace(/["\\;\n\r]/g, '')}", system-ui, sans-serif`;
	}

	function glyphOutlineCacheKey(
		faceId: string,
		codepoint: number,
		variations: FontGlyphVariationValue[]
	): string {
		return `${faceId}:${codepoint}:${variations
			.map((variation) => `${variation.tag}=${variation.value.toFixed(3)}`)
			.join(',')}`;
	}

	function nearestWeight(weights: number[], target: number): number {
		return weights.reduce(
			(closest, weight) =>
				Math.abs(weight - target) < Math.abs(closest - target) ? weight : closest,
			weights[0] ?? 400
		);
	}

	let availableWeights = $derived(
		family?.weights.length ? [...family.weights].sort((a, b) => a - b) : [400]
	);
	let nameWeight = $derived(nearestWeight(availableWeights, 600));
	let glyphWeight = $derived(glyphWeightOverride ?? previewWeight);
	let selectedFace = $derived.by(() => {
		if (!family) return null;
		return (
			family.faces.find((face) => face.id === selectedFaceOverrideId) ??
			family.faces.find((face) => face.weight === glyphWeight && face.style === 'normal') ??
			family.faces.find((face) => face.weight === glyphWeight) ??
			family.faces[0] ??
			null
		);
	});
	let selectedGlyphCodepoint = $derived(selectedGlyph.codePointAt(0) ?? 0);
	let selectedCodepoint = $derived(formatCodepoint(selectedGlyphCodepoint));
	let visibleGlyphCodepoints = $derived(
		filterGlyphSetCodepoints(faceInspection?.unicodeCodepoints ?? [], glyphSetScope)
	);
	let glyphCategories = $derived(groupUnicodeCodepoints(visibleGlyphCodepoints));
	let glyphVariationValues = $derived.by((): FontGlyphVariationValue[] => {
		return (faceInspection?.variationAxes ?? []).map((axis) => ({
			tag: axis.tag,
			value:
				axis.tag === 'wght'
					? Math.min(axis.maximum, Math.max(axis.minimum, glyphWeight))
					: axis.default
		}));
	});
	let displayText = $derived(previewText.trim() || family?.name || 'Type to preview');
	let caseTransform = $derived(
		letterCase === 'upper' ? 'uppercase' : letterCase === 'lower' ? 'lowercase' : 'none'
	);
	let metricGuides = $derived.by(() => {
		const metrics = faceInspection?.metrics;
		if (!metrics) return [];
		return [
			{
				label: 'Cap height',
				value: metrics.capitalHeight,
				source: metrics.capitalHeightSource
			},
			{ label: 'X-height', value: metrics.xHeight, source: metrics.xHeightSource },
			{ label: 'Baseline', value: metrics.baseline, source: 'font-metric' },
			{ label: 'Descender', value: metrics.descender, source: 'font-metric' }
		].filter(
			(guide): guide is { label: string; value: number; source: string } =>
				guide.value !== null
		);
	});
	let hasDerivedMetricGuides = $derived(
		metricGuides.some((guide) => guide.source === 'glyph-bounds')
	);
	let metricChart = $derived.by(() => {
		const metrics = faceInspection?.metrics;
		if (!metrics) return null;
		const top = Math.max(metrics.ascender, metrics.capitalHeight ?? 0, metrics.xHeight ?? 0, 0);
		const bottom = Math.min(metrics.descender, 0);
		const padding = Math.max(80, metrics.unitsPerEm * 0.14);
		return {
			width: metrics.unitsPerEm * 1.35,
			height: top - bottom + padding * 2,
			fontSize: metrics.unitsPerEm,
			labelSize: Math.max(34, metrics.unitsPerEm * 0.042),
			y: (value: number) => top + padding - value
		};
	});
	let glyphOutlineChart = $derived.by(() => {
		if (!metricChart || !glyphOutline) return null;
		const advanceWidth = glyphOutline.advanceWidth ?? glyphOutline.unitsPerEm;
		return {
			originX: (metricChart.width - advanceWidth) / 2,
			advanceWidth,
			pointRadius: Math.max(9, glyphOutline.unitsPerEm * 0.011),
			transform: `translate(${(metricChart.width - advanceWidth) / 2} ${metricChart.y(0)}) scale(1 -1)`
		};
	});

	let heroStyle = $derived(
		family
			? `font-family: ${safeFontStack(family.name)}; font-size: clamp(40px, ${previewSize}px, ${previewSize}px); font-weight: ${previewWeight}; text-align: ${alignment}; text-transform: ${caseTransform};`
			: ''
	);

	function faceStyle(weight: number, style: string): string {
		if (!family) return '';
		return `font-family: ${safeFontStack(family.name)}; font-weight: ${weight}; font-style: ${
			style === 'italic' ? 'italic' : 'normal'
		}; font-size: ${stylesSize}px; text-align: ${alignment}; text-transform: ${caseTransform};`;
	}

	// Mirror the shared preview text into the editable specimen, but never while
	// it holds the caret — writing back mid-edit would jump the cursor.
	$effect(() => {
		const text = previewText;
		if (
			specimenEl &&
			document.activeElement !== specimenEl &&
			specimenEl.textContent !== text
		) {
			specimenEl.textContent = text;
		}
	});

	$effect(() => {
		const faceId = selectedFace?.id;
		faceInspection = null;
		inspectionError = '';
		parserExport = null;
		parserError = '';
		parserCopyLabel = 'Copy JSON';
		if (!faceId) return;
		if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
			inspectionError = 'Metric data is available in the desktop app.';
			return;
		}

		let cancelled = false;
		inspectionLoading = true;
		void inspectFontFace(faceId)
			.then((inspection) => {
				if (!cancelled && inspection.faceId === faceId) faceInspection = inspection;
			})
			.catch((error: unknown) => {
				if (!cancelled) inspectionError = commandErrorMessage(error);
			})
			.finally(() => {
				if (!cancelled) inspectionLoading = false;
			});

		return () => {
			cancelled = true;
		};
	});

	$effect(() => {
		const mode = glyphViewMode;
		const faceId = selectedFace?.id;
		const inspectedFaceId = faceInspection?.faceId;
		const codepoint = selectedGlyphCodepoint;
		const variations = glyphVariationValues;

		if (mode === 'fill') {
			glyphOutlineLoading = false;
			glyphOutlineError = '';
			return;
		}

		glyphOutline = null;
		glyphOutlineError = '';
		if (!faceId || inspectedFaceId !== faceId) return;
		if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
			glyphOutlineError = 'Outline data is available in the desktop app.';
			return;
		}

		const cacheKey = glyphOutlineCacheKey(faceId, codepoint, variations);
		const cached = glyphOutlineCache.get(cacheKey);
		if (cached) {
			glyphOutline = cached;
			glyphOutlineLoading = false;
			return;
		}

		let cancelled = false;
		const timer = window.setTimeout(() => {
			glyphOutlineLoading = true;
			void inspectFontGlyphOutline({ faceId, codepoint, variations })
				.then((outline) => {
					if (cancelled) return;
					glyphOutlineCache.set(cacheKey, outline);
					glyphOutline = outline;
				})
				.catch((error: unknown) => {
					if (!cancelled) glyphOutlineError = commandErrorMessage(error);
				})
				.finally(() => {
					if (!cancelled) glyphOutlineLoading = false;
				});
		}, 70);

		return () => {
			cancelled = true;
			window.clearTimeout(timer);
		};
	});

	$effect(() => {
		if (typeof window === 'undefined') return;
		const query = window.matchMedia('(prefers-reduced-motion: reduce)');
		const syncPreference = () => (prefersReducedMotion = query.matches);
		syncPreference();
		query.addEventListener('change', syncPreference);
		return () => query.removeEventListener('change', syncPreference);
	});

	$effect(() => {
		const faceId = faceInspection?.faceId;
		const coverageKey = faceId ? `${faceId}:${glyphSetScope}` : null;
		if (!coverageKey || coverageKey === glyphCoverageFaceId) return;
		glyphCoverageFaceId = coverageKey;
		expandedGlyphCategories = new Set();
		glyphCategoryLimits = {};
	});

	// Shadow-lift the sticky control bar once the hero scrolls past it.
	$effect(() => {
		const sentinel = sentinelEl;
		if (!sentinel) return;
		const observer = new IntersectionObserver(
			([entry]) => {
				stuck = !entry.isIntersecting;
			},
			{ threshold: 0 }
		);
		observer.observe(sentinel);
		return () => observer.disconnect();
	});

	function handleSpecimenInput() {
		if (specimenEl) onPreviewText(specimenEl.textContent ?? '');
	}

	function handleSpecimenKeydown(event: KeyboardEvent) {
		// A tester is single-line in spirit; keep newlines out of the specimen.
		if (event.key === 'Enter') event.preventDefault();
	}

	function focusSpecimen() {
		specimenEl?.focus();
		const selection = window.getSelection();
		if (selection && specimenEl) {
			selection.selectAllChildren(specimenEl);
			selection.collapseToEnd();
		}
	}

	function setGlyphWeight(weight: number) {
		selectedFaceOverrideId = null;
		glyphWeightOverride = weight;
	}

	function selectInspectionFace(faceId: string) {
		const face = family?.faces.find((candidate) => candidate.id === faceId);
		if (!face) return;
		selectedFaceOverrideId = face.id;
		glyphWeightOverride = face.weight;
	}

	function previewGlyph(glyph: string) {
		if (lockedGlyph === null) selectedGlyph = glyph;
	}

	function toggleGlyphLock(glyph: string) {
		if (lockedGlyph === glyph) {
			lockedGlyph = null;
			return;
		}
		lockedGlyph = glyph;
		selectedGlyph = glyph;
	}

	function toggleGlyphCategory(key: string) {
		const next = new Set(expandedGlyphCategories);
		if (next.has(key)) {
			next.delete(key);
		} else {
			next.add(key);
			if (glyphCategoryLimits[key] === undefined) {
				glyphCategoryLimits = { ...glyphCategoryLimits, [key]: INITIAL_GLYPH_BATCH };
			}
		}
		expandedGlyphCategories = next;
	}

	function showMoreGlyphs(key: string, total: number) {
		const current = glyphCategoryLimits[key] ?? INITIAL_GLYPH_BATCH;
		glyphCategoryLimits = {
			...glyphCategoryLimits,
			[key]: Math.min(total, current + GLYPH_BATCH_SIZE)
		};
	}

	function showAllGlyphs(key: string, total: number) {
		glyphCategoryLimits = { ...glyphCategoryLimits, [key]: total };
	}

	function glyphAccessibleName(codepoint: number): string {
		const display = glyphCellText(codepoint);
		const formatted = formatCodepoint(codepoint);
		return display === formatted ? formatted : `${display}, ${formatted}`;
	}

	async function loadParserJson() {
		const faceId = selectedFace?.id;
		if (!faceId || parserExport?.faceId === faceId || parserLoading) return;
		if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
			parserError = 'Parser JSON is available in the desktop app.';
			return;
		}

		parserLoading = true;
		parserError = '';
		try {
			const exported = await exportFontFaceParserJson(faceId);
			if (selectedFace?.id === faceId) parserExport = exported;
		} catch (error) {
			parserError = commandErrorMessage(error);
		} finally {
			parserLoading = false;
		}
	}

	async function copyParserJson() {
		if (!parserExport) return;
		try {
			await navigator.clipboard.writeText(parserExport.rawJson);
			parserCopyLabel = 'Copied';
			window.setTimeout(() => (parserCopyLabel = 'Copy JSON'), 1600);
		} catch {
			parserCopyLabel = 'Copy failed';
		}
	}

	function commandErrorMessage(error: unknown): string {
		if (typeof error === 'object' && error && 'message' in error) {
			return String(error.message);
		}
		return 'FontNest could not inspect this font face.';
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function formatInteger(value: number): string {
		return new Intl.NumberFormat().format(value);
	}

	function widthClassName(width: number): string {
		return (
			[
				'',
				'Ultra-condensed',
				'Extra-condensed',
				'Condensed',
				'Semi-condensed',
				'Normal',
				'Semi-expanded',
				'Expanded',
				'Extra-expanded',
				'Ultra-expanded'
			][width] ?? `Class ${width}`
		);
	}

	function embeddingLabel(value: string | null): string {
		if (!value) return 'Not specified';
		return value
			.replace('PreviewAndPrint', 'Preview & print')
			.replace(/([a-z])([A-Z])/g, '$1 $2');
	}

	function formatAxisValue(value: number): string {
		return Number.isInteger(value) ? String(value) : value.toFixed(2).replace(/0+$/, '').replace(/\.$/, '');
	}
</script>

<section class="font-preview-page" aria-labelledby="font-preview-title">
	{#if family}
		<div class="page-inner">
			<header class="preview-header">
				<button type="button" class="back-action" onclick={onBack}>
					<span class="back-icon"><Icon name="chevron" size={15} /></span>
					Library
				</button>

				<div class="heading-row">
					<div class="heading-copy">
						<h1
							id="font-preview-title"
							class="family-name"
							style={`font-family: ${safeFontStack(family.name)}; font-weight: ${nameWeight};`}
						>
							{family.name}
						</h1>
						<ul class="fact-chips">
							{#each family.sources as source (source)}
								<li>{source}</li>
							{/each}
							{#each family.formats as format (format)}
								<li>{format}</li>
							{/each}
							<li>{family.faceCount} {family.faceCount === 1 ? 'face' : 'faces'}</li>
							<li>{family.fileCount} {family.fileCount === 1 ? 'file' : 'files'}</li>
							<li>{family.monospaced ? 'Monospaced' : 'Proportional'}</li>
							{#if family.hasConflict}
								<li class="chip-warning">
									<Icon name="alert" size={12} /> Conflict
								</li>
							{/if}
						</ul>
					</div>
					<button
						type="button"
						class:pinned
						class="sidebar-action"
						aria-pressed={pinned}
						onclick={onTogglePinned}
					>
						<Icon name="bookmark" size={16} />
						<span>{pinned ? 'Saved' : 'Save to sidebar'}</span>
					</button>
				</div>
			</header>

			<span bind:this={sentinelEl} class="scroll-sentinel" aria-hidden="true"></span>

			<div class:stuck class="tester-controls">
				<div class="weight-scroller" role="group" aria-label="Preview weight">
					{#each availableWeights as weight (weight)}
						<button
							type="button"
							class:active={previewWeight === weight}
							aria-pressed={previewWeight === weight}
							onclick={() => onPreviewWeight(weight)}
						>
							{weightName(weight)}
							<small>{weight}</small>
						</button>
					{/each}
				</div>

				<div class="control-cluster">
					<label class="size-control">
						<span class="control-label">Size</span>
						<input
							type="range"
							min="16"
							max="200"
							step="1"
							value={previewSize}
							aria-label="Preview size in pixels"
							oninput={(event) => onPreviewSize(Number(event.currentTarget.value))}
						/>
						<output>{previewSize}px</output>
					</label>

					<div class="segmented" role="group" aria-label="Text alignment">
						<button
							type="button"
							class:active={alignment === 'left'}
							aria-pressed={alignment === 'left'}
							aria-label="Align left"
							onclick={() => (alignment = 'left')}
						>
							<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
								<path
									d="M4 6.5h16M4 11h10M4 15.5h13"
									fill="none"
									stroke="currentColor"
									stroke-width="1.7"
									stroke-linecap="round"
								/>
							</svg>
						</button>
						<button
							type="button"
							class:active={alignment === 'center'}
							aria-pressed={alignment === 'center'}
							aria-label="Align center"
							onclick={() => (alignment = 'center')}
						>
							<svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
								<path
									d="M4 6.5h16M7 11h10M6 15.5h12"
									fill="none"
									stroke="currentColor"
									stroke-width="1.7"
									stroke-linecap="round"
								/>
							</svg>
						</button>
					</div>

					<div class="segmented text-segmented" role="group" aria-label="Letter case">
						<button
							type="button"
							class:active={letterCase === 'as-typed'}
							aria-pressed={letterCase === 'as-typed'}
							onclick={() => (letterCase = 'as-typed')}>Aa</button
						>
						<button
							type="button"
							class:active={letterCase === 'upper'}
							aria-pressed={letterCase === 'upper'}
							onclick={() => (letterCase = 'upper')}>AA</button
						>
						<button
							type="button"
							class:active={letterCase === 'lower'}
							aria-pressed={letterCase === 'lower'}
							onclick={() => (letterCase = 'lower')}>aa</button
						>
					</div>
				</div>
			</div>

			<div class="tester">
				<!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
				<p
					bind:this={specimenEl}
					class="specimen"
					style={heroStyle}
					contenteditable="plaintext-only"
					role="textbox"
					aria-multiline="false"
					aria-label={`Editable ${family.name} preview. Type to change the sample text.`}
					tabindex="0"
					spellcheck="false"
					data-placeholder="Type to preview…"
					oninput={handleSpecimenInput}
					onkeydown={handleSpecimenKeydown}
				></p>
				<button type="button" class="edit-hint" onclick={focusSpecimen}>
					<Icon name="font" size={13} /> Click to edit the sample
				</button>
			</div>

			<section class="styles-section" aria-labelledby="styles-title">
				<div class="section-head">
					<h2 id="styles-title">
						Styles <span class="count">{family.faceCount}</span>
					</h2>
					<label class="size-control compact">
						<span class="control-label">Row size</span>
						<input
							type="range"
							min="20"
							max="96"
							step="1"
							value={stylesSize}
							aria-label="Style row size in pixels"
							oninput={(event) => (stylesSize = Number(event.currentTarget.value))}
						/>
						<output>{stylesSize}px</output>
					</label>
				</div>

				<ul class="style-list">
					{#each family.faces as face, index (face.id)}
						<li class="style-row" style={`--row-index: ${index};`}>
							<div class="style-meta">
								<strong>{face.styleName || weightName(face.weight)}</strong>
								<span class="style-tags">
									<span>{face.weight}</span>
									{#if face.style === 'italic'}<span class="tag-italic"
											>Italic</span
										>{/if}
									<span class="tag-format">{face.format}</span>
								</span>
							</div>
							<p class="style-specimen" style={faceStyle(face.weight, face.style)}>
								{displayText}
							</p>
						</li>
					{/each}
				</ul>
			</section>

			<section class="glyphs-section" aria-labelledby="glyphs-title">
				<div class="section-head">
					<h2 id="glyphs-title">
						Characters
						{#if faceInspection}
							<span
								class="count"
								title={glyphSetScope === 'basic'
									? `${formatInteger(visibleGlyphCodepoints.length)} basic characters from ${formatInteger(faceInspection.unicodeCodepoints.length)} total`
									: `${formatInteger(faceInspection.unicodeCodepoints.length)} total characters`}
							>
								{formatInteger(visibleGlyphCodepoints.length)}
								{#if glyphSetScope === 'basic'}
									<span class="count-total">/ {formatInteger(faceInspection.unicodeCodepoints.length)}</span>
								{/if}
							</span>
						{/if}
					</h2>
					<div class="character-controls">
						<div class="glyph-set-switcher" role="group" aria-label="Character set size">
							{#each GLYPH_SET_SCOPES as scope (scope.value)}
								<button
									type="button"
									class:active={glyphSetScope === scope.value}
									aria-pressed={glyphSetScope === scope.value}
									onclick={() => (glyphSetScope = scope.value)}
								>
									{scope.label}
								</button>
							{/each}
						</div>
						<div
							class="weight-scroller mini"
							role="group"
							aria-label="Character weight"
						>
							{#each availableWeights as weight (weight)}
								<button
									type="button"
									class:active={glyphWeight === weight}
									aria-pressed={glyphWeight === weight}
									onclick={() => setGlyphWeight(weight)}
								>
									{weight}
								</button>
							{/each}
						</div>
						{#if family.faces.length > 1}
							<label class="face-control">
								<span>Face</span>
								<select
									value={selectedFace?.id ?? ''}
									onchange={(event) =>
										selectInspectionFace(event.currentTarget.value)}
								>
									{#each family.faces as face (face.id)}
										<option value={face.id}
											>{face.styleName} · {face.weight}</option
										>
									{/each}
								</select>
							</label>
						{/if}
					</div>
				</div>

				<div class="glyph-layout">
					<div class="glyph-catalogue">
						{#if inspectionLoading}
							<div class="glyph-coverage-loading" aria-live="polite">
								<span></span><span></span><span></span>
								<p>Reading every mapped character in this face…</p>
							</div>
						{:else if inspectionError}
							<p class="glyph-coverage-message">{inspectionError}</p>
						{:else if glyphCategories.length}
							<div class="glyph-accordions">
								{#each glyphCategories as group (group.key)}
									{@const expanded = expandedGlyphCategories.has(group.key)}
									{@const limit = glyphCategoryLimits[group.key] ?? INITIAL_GLYPH_BATCH}
									<section class:expanded class="glyph-accordion">
										<button
											type="button"
											class="glyph-accordion-trigger"
											aria-expanded={expanded}
											aria-controls={`glyph-panel-${group.key}`}
											onclick={() => toggleGlyphCategory(group.key)}
										>
											<span class="glyph-category-copy">
												<strong>{group.label}</strong>
												<small>{group.description}</small>
											</span>
											<span class="glyph-category-count">{formatInteger(group.codepoints.length)}</span>
											<span class="glyph-accordion-chevron" aria-hidden="true"></span>
										</button>

										{#if expanded}
											<div
												id={`glyph-panel-${group.key}`}
												class="glyph-accordion-panel"
												in:slide={{ duration: prefersReducedMotion ? 0 : 300, easing: quintOut }}
												out:slide={{ duration: prefersReducedMotion ? 0 : 220, easing: quintOut }}
											>
												<div
													class="glyphs"
													style={`font-family: ${safeFontStack(family.name)}; font-weight: ${glyphWeight}; font-style: ${selectedFace?.style ?? 'normal'};`}
												>
													{#each group.codepoints.slice(0, limit) as codepoint (codepoint)}
														{@const glyph = String.fromCodePoint(codepoint)}
														<button
															type="button"
															class:active={selectedGlyph === glyph}
															class:locked={lockedGlyph === glyph}
															class:codepoint-placeholder={usesCodepointPlaceholder(codepoint)}
															aria-pressed={lockedGlyph === glyph}
															aria-label={lockedGlyph === glyph
																? `Unlock ${glyphAccessibleName(codepoint)} from the character preview`
																: `Lock ${glyphAccessibleName(codepoint)} in the character preview`}
															title={`${formatCodepoint(codepoint)} · ${lockedGlyph === glyph ? 'Click to unlock hover preview' : 'Click to lock in the preview'}`}
															onclick={() => toggleGlyphLock(glyph)}
															onmouseenter={() => previewGlyph(glyph)}
															onfocus={() => previewGlyph(glyph)}
														>
															{glyphCellText(codepoint)}
														</button>
													{/each}
												</div>

												{#if limit < group.codepoints.length}
													<div class="glyph-batch-actions">
														<p>
															Showing {formatInteger(Math.min(limit, group.codepoints.length))} of
															{formatInteger(group.codepoints.length)}
														</p>
														<button type="button" onclick={() => showMoreGlyphs(group.key, group.codepoints.length)}>
															Show next {formatInteger(Math.min(GLYPH_BATCH_SIZE, group.codepoints.length - limit))}
														</button>
														<button type="button" onclick={() => showAllGlyphs(group.key, group.codepoints.length)}>
															Show all
														</button>
													</div>
												{/if}
											</div>
										{/if}
									</section>
								{/each}
							</div>
						{:else}
							<p class="glyph-coverage-message">This face does not expose Unicode-mapped characters.</p>
						{/if}
					</div>

					<aside class="glyph-detail" aria-live="polite">
						<div class="glyph-detail-inner">
							<div class="glyph-view-toolbar">
								<span>View</span>
								<div class="glyph-view-switcher" role="group" aria-label="Glyph drawing mode">
									{#each GLYPH_VIEW_MODES as mode (mode.value)}
										<button
											type="button"
											class:active={glyphViewMode === mode.value}
											aria-pressed={glyphViewMode === mode.value}
											onclick={() => (glyphViewMode = mode.value)}
										>
											{mode.label}
										</button>
									{/each}
								</div>
							</div>
							{#if metricChart && faceInspection}
								<div class:tracing={glyphOutlineLoading} class="metric-chart">
									<svg
										viewBox={`0 0 ${metricChart.width} ${metricChart.height}`}
										role="img"
										aria-label={`${selectedGlyph} ${glyphViewMode} view aligned to the font's cap height, x-height, baseline, and descender`}
										preserveAspectRatio="xMidYMid meet"
									>
										{#each metricGuides as guide (guide.label)}
											<g
												class="metric-guide"
												class:derived={guide.source === 'glyph-bounds'}
											>
												<line
													x1="0"
													x2={metricChart.width}
													y1={metricChart.y(guide.value)}
													y2={metricChart.y(guide.value)}
												/>
												<text
													class="metric-label"
													x={metricChart.labelSize * 0.35}
													y={metricChart.y(guide.value) -
														metricChart.labelSize * 0.32}
													style={`font-size: ${metricChart.labelSize}px;`}
													>{guide.label}{guide.source === 'glyph-bounds'
														? '*'
														: ''}</text
												>
												<text
													class="metric-value"
													x={metricChart.width -
														metricChart.labelSize * 0.35}
													y={metricChart.y(guide.value) -
														metricChart.labelSize * 0.32}
													text-anchor="end"
													style={`font-size: ${metricChart.labelSize}px;`}
													>{guide.value}</text
												>
											</g>
										{/each}

										{#if glyphViewMode === 'fill' || !glyphOutline?.outlineAvailable || !glyphOutlineChart}
											<text
												class:fallback={glyphViewMode !== 'fill'}
												class="metric-glyph"
												x={metricChart.width / 2}
												y={metricChart.y(0)}
												text-anchor="middle"
												style={`font-family: ${safeFontStack(family.name)}; font-weight: ${glyphWeight}; font-style: ${selectedFace?.style ?? 'normal'}; font-size: ${metricChart.fontSize}px;`}
											>
												{selectedGlyph}
											</text>
										{:else}
											<g
												class:show-points={glyphViewMode === 'points'}
												class="glyph-outline-layer"
												transform={glyphOutlineChart.transform}
											>
												{#if glyphViewMode === 'points'}
													<g class="glyph-dimensions" aria-hidden="true">
														<line
															x1="0"
															x2="0"
															y1={faceInspection.metrics.descender}
															y2={faceInspection.metrics.ascender}
														/>
														<line
															x1={glyphOutlineChart.advanceWidth}
															x2={glyphOutlineChart.advanceWidth}
															y1={faceInspection.metrics.descender}
															y2={faceInspection.metrics.ascender}
														/>
														{#if glyphOutline.bounds}
															<rect
																x={glyphOutline.bounds.xMin}
																y={glyphOutline.bounds.yMin}
																width={glyphOutline.bounds.xMax - glyphOutline.bounds.xMin}
																height={glyphOutline.bounds.yMax - glyphOutline.bounds.yMin}
															/>
														{/if}
													</g>
													<g class="glyph-handles" aria-hidden="true">
														{#each glyphOutline.handles as handle, index (`${index}-${handle.x1}-${handle.y1}`)}
															<line x1={handle.x1} y1={handle.y1} x2={handle.x2} y2={handle.y2} />
														{/each}
													</g>
												{/if}
												<path class="glyph-outline-path" d={glyphOutline.pathData} />
												{#if glyphViewMode === 'points'}
													<g class="glyph-outline-points" aria-hidden="true">
														{#each glyphOutline.points as point, index (`${index}-${point.x}-${point.y}`)}
															<circle
																class={point.kind}
																cx={point.x}
																cy={point.y}
																r={glyphOutlineChart.pointRadius}
															/>
														{/each}
													</g>
												{/if}
											</g>
										{/if}
									</svg>
									{#if glyphViewMode !== 'fill' && (glyphOutlineLoading || glyphOutlineError || (glyphOutline && !glyphOutline.outlineAvailable))}
										<p class="glyph-outline-status">
											{glyphOutlineLoading
												? 'Tracing glyph outlineâ€¦'
												: glyphOutlineError || 'No vector outline â€” showing the filled glyph.'}
										</p>
									{/if}
								</div>
							{:else}
								<div
									class="glyph-big"
									style={`font-family: ${safeFontStack(family.name)}; font-weight: ${glyphWeight}; font-style: ${selectedFace?.style ?? 'normal'};`}
								>
									{selectedGlyph}
									{#if inspectionLoading}
										<span class="metric-status">Reading font metrics…</span>
									{:else if inspectionError}
										<span class="metric-status">{inspectionError}</span>
									{/if}
								</div>
							{/if}
							<dl class="glyph-facts">
								<div class="selection-state">
									<dt>Selection</dt>
									<dd>{lockedGlyph === null ? 'Hover preview' : 'Locked'}</dd>
								</div>
								<div>
									<dt>Unicode</dt>
									<dd>{selectedCodepoint}</dd>
								</div>
								<div>
									<dt>Weight</dt>
									<dd>{glyphWeight}</dd>
								</div>
								{#if selectedFace}
									<div>
										<dt>Face</dt>
										<dd>{selectedFace.styleName}</dd>
									</div>
								{/if}
								{#if faceInspection}
									<div>
										<dt>Units per em</dt>
										<dd>{faceInspection.metrics.unitsPerEm}</dd>
									</div>
								{/if}
								{#if glyphViewMode !== 'fill'}
									<div>
										<dt>Outline</dt>
										<dd>
											{glyphOutlineLoading
												? 'Reading'
												: glyphOutline?.outlineAvailable
													? `${glyphOutline.contourCount} contours`
													: 'Unavailable'}
										</dd>
									</div>
									{#if glyphOutline?.outlineAvailable}
										<div>
											<dt>Points</dt>
											<dd>{formatInteger(glyphOutline.points.length)}</dd>
										</div>
										{#if glyphOutline.advanceWidth !== null}
											<div>
												<dt>Advance</dt>
												<dd>{glyphOutline.advanceWidth}</dd>
											</div>
										{/if}
										{#if glyphOutline.leftSideBearing !== null}
											<div>
												<dt>Left bearing</dt>
												<dd>{glyphOutline.leftSideBearing}</dd>
											</div>
										{/if}
									{/if}
								{/if}
								{#if hasDerivedMetricGuides}
									<div class="derived-note">
										<dt>* Guide</dt>
										<dd>Measured from H/x</dd>
									</div>
								{/if}
							</dl>
						</div>
					</aside>
				</div>
			</section>

			<section class="details-section" aria-labelledby="details-title">
				<div class="details-heading">
					<h2 id="details-title">Family details</h2>
					{#if selectedFace}
						<p>{selectedFace.styleName} · parsed face {selectedFace.faceIndex}</p>
					{/if}
				</div>
				<div class="details-grid">
					<div class="parsed-summary">
						{#if inspectionLoading}
							<div class="details-loading" aria-live="polite">
								<span></span><span></span><span></span><span></span>
								<p>Reading selected face details…</p>
							</div>
						{:else if inspectionError}
							<p class="details-error">{inspectionError}</p>
						{:else if faceInspection && selectedFace}
							<dl class="summary">
								{#if faceInspection.names.fullName}
									<div>
										<dt>Full name</dt>
										<dd>{faceInspection.names.fullName}</dd>
									</div>
								{/if}
								<div>
									<dt>PostScript name</dt>
									<dd>{selectedFace.postScriptName || 'Not provided'}</dd>
								</div>
								{#if faceInspection.names.version}
									<div>
										<dt>Version</dt>
										<dd>{faceInspection.names.version}</dd>
									</div>
								{/if}
								<div>
									<dt>Coverage</dt>
									<dd>
										{formatInteger(faceInspection.properties.glyphCount)} glyphs ·
										{formatInteger(faceInspection.properties.unicodeCodepointCount)} characters
									</dd>
								</div>
								<div>
									<dt>OpenType tables</dt>
									<dd>{faceInspection.properties.tableCount}</dd>
								</div>
								<div>
									<dt>Design space</dt>
									<dd>
										{faceInspection.variationAxes.length
											? `${faceInspection.variationAxes.length} variation ${faceInspection.variationAxes.length === 1 ? 'axis' : 'axes'}`
											: 'Static face'}
									</dd>
								</div>
								<div>
									<dt>Width class</dt>
									<dd>{widthClassName(faceInspection.properties.width)}</dd>
								</div>
								<div>
									<dt>Face traits</dt>
									<dd>{faceInspection.properties.traits.join(' · ') || 'None reported'}</dd>
								</div>
								{#if faceInspection.properties.italicAngle !== 0}
									<div>
										<dt>Italic angle</dt>
										<dd>{formatAxisValue(faceInspection.properties.italicAngle)}°</dd>
									</div>
								{/if}
								<div>
									<dt>Embedding</dt>
									<dd>
										{embeddingLabel(faceInspection.properties.embedding.permissions)} ·
										{faceInspection.properties.embedding.subsettingAllowed
											? 'subsetting allowed'
											: 'no subsetting'}
									</dd>
								</div>
							</dl>

							<dl class="metric-summary" aria-label="Selected face metrics">
								<div><dt>UPM</dt><dd>{faceInspection.metrics.unitsPerEm}</dd></div>
								<div><dt>Ascender</dt><dd>{faceInspection.metrics.ascender}</dd></div>
								<div>
									<dt>Cap height</dt>
									<dd>{faceInspection.metrics.capitalHeight ?? '—'}</dd>
								</div>
								<div>
									<dt>X-height</dt>
									<dd>{faceInspection.metrics.xHeight ?? '—'}</dd>
								</div>
								<div><dt>Descender</dt><dd>{faceInspection.metrics.descender}</dd></div>
								<div><dt>Line gap</dt><dd>{faceInspection.metrics.lineGap}</dd></div>
							</dl>

							{#if faceInspection.variationAxes.length}
								<div class="axis-summary">
									<h3>Variation axes</h3>
									<ul>
										{#each faceInspection.variationAxes as axis (axis.tag)}
											<li>
												<strong>{axis.tag}</strong>
												<span>{formatAxisValue(axis.minimum)} — {formatAxisValue(axis.maximum)}</span>
												<small>default {formatAxisValue(axis.default)}</small>
											</li>
										{/each}
									</ul>
								</div>
							{/if}

							{#if faceInspection.names.manufacturer || faceInspection.names.designer || faceInspection.names.license}
								<dl class="credits-summary">
									{#if faceInspection.names.manufacturer}
										<div><dt>Manufacturer</dt><dd>{faceInspection.names.manufacturer}</dd></div>
									{/if}
									{#if faceInspection.names.designer}
										<div><dt>Designer</dt><dd>{faceInspection.names.designer}</dd></div>
									{/if}
									{#if faceInspection.names.license}
										<div>
											<dt>License</dt>
											<dd class="license-copy" title={faceInspection.names.license}>
												{faceInspection.names.license}
											</dd>
										</div>
									{/if}
								</dl>
							{/if}
						{:else}
							<p class="details-error">Select a face to inspect its parsed details.</p>
						{/if}
					</div>

					<div class="faces-table" role="table" aria-label="Installed faces">
						<div class="faces-head" role="row">
							<span role="columnheader">Style</span>
							<span role="columnheader">Weight</span>
							<span role="columnheader">File</span>
						</div>
						{#each family.faces as face (face.id)}
							<div
								class="faces-row"
								class:selected={selectedFace?.id === face.id}
								role="row"
								aria-current={selectedFace?.id === face.id ? 'true' : undefined}
							>
								<span role="cell" class="face-style">
									{face.styleName || weightName(face.weight)}
									{#if face.style === 'italic'}<em>Italic</em>{/if}
									{#if selectedFace?.id === face.id}<small class="selected-face-label"
										>Selected</small
									>{/if}
								</span>
								<span role="cell" class="face-weight">{face.weight}</span>
								<span role="cell" class="face-file" title={face.fileName}
									>{face.fileName}</span
								>
							</div>
						{/each}
					</div>
				</div>
			</section>

			<section class="parser-section" aria-labelledby="parser-title">
				<details
					ontoggle={(event) => {
						if (event.currentTarget.open) void loadParserJson();
					}}
				>
					<summary>
						<span class="parser-heading">
							<strong id="parser-title">Parser JSON</strong>
							<small>
								{selectedFace?.styleName ?? 'No face selected'} · {faceInspection?.parserName ??
									'ttf-parser'}
							</small>
						</span>
						<span class="parser-disclosure">View raw data</span>
					</summary>

					<div class="parser-content">
						{#if parserLoading}
							<div class="parser-loading" aria-live="polite">
								<span></span><span></span><span></span>
								<p>Exporting tables, names, Unicode mappings, and glyph metrics…</p>
							</div>
						{:else if parserError}
							<p class="parser-error">{parserError}</p>
						{:else if parserExport}
							<div class="parser-toolbar">
								<p>
									{parserExport.parserName}
									{parserExport.parserVersion} ·
									{formatBytes(parserExport.jsonByteLength)}
								</p>
								<button type="button" onclick={copyParserJson}
									>{parserCopyLabel}</button
								>
							</div>
							<div
								class="parser-json"
								role="region"
								aria-label="Raw font parser JSON"
							>
								<pre><code>{parserExport.rawJson}</code></pre>
							</div>
						{:else}
							<button
								type="button"
								class="load-parser-action"
								onclick={loadParserJson}
							>
								Load parser JSON
							</button>
						{/if}
					</div>
				</details>
			</section>
		</div>
	{:else}
		<div class="missing-preview">
			<div class="missing-icon"><Icon name="font" size={24} /></div>
			<h1 id="font-preview-title">This font is no longer available</h1>
			<p>Return to the library and choose another family to preview.</p>
			<button type="button" onclick={onBack}>Return to library</button>
		</div>
	{/if}
</section>

<style>
	.font-preview-page {
		min-height: 100%;
		background: var(--color-surface);
	}

	.page-inner {
		max-width: 1200px;
		margin: 0 auto;
		padding: 24px clamp(20px, 3vw, 40px) 56px;
	}

	/* Header */
	.preview-header {
		padding-bottom: 8px;
	}

	.back-action,
	.sidebar-action,
	.missing-preview button {
		display: inline-flex;
		min-height: 36px;
		align-items: center;
		gap: 7px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
		transition:
			color var(--motion-fast),
			background var(--motion-fast),
			border-color var(--motion-fast),
			transform var(--motion-fast);
	}

	.back-action {
		padding: 0 10px 0 8px;
		border-color: transparent;
		color: var(--color-muted);
		background: transparent;
	}

	.back-icon {
		display: inline-flex;
		transform: rotate(180deg);
	}

	.back-action:hover,
	.sidebar-action:hover,
	.missing-preview button:hover {
		color: var(--color-text);
		background: var(--color-hover);
	}

	.back-action:active,
	.sidebar-action:active,
	.missing-preview button:active {
		transform: translateY(1px);
	}

	.heading-row {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 24px;
		margin-top: 14px;
	}

	.heading-copy {
		min-width: 0;
	}

	.family-name {
		margin: 0;
		font-size: clamp(2.4rem, 7vw, 4.5rem);
		line-height: 1.02;
		letter-spacing: -0.03em;
		text-wrap: balance;
		overflow-wrap: anywhere;
		font-optical-sizing: auto;
	}

	.fact-chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin: 16px 0 0;
		padding: 0;
		list-style: none;
	}

	.fact-chips li {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		height: 24px;
		padding: 0 10px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-muted);
		background: var(--color-panel);
		font-size: var(--text-micro);
		font-weight: 600;
		white-space: nowrap;
	}

	.fact-chips .chip-warning {
		color: var(--color-warning);
		border-color: color-mix(in srgb, var(--color-warning) 40%, var(--color-border));
	}

	.sidebar-action {
		flex: none;
		padding: 0 12px;
	}

	.sidebar-action.pinned {
		border-color: color-mix(in srgb, var(--color-accent) 58%, var(--color-border));
		background: color-mix(in srgb, var(--color-accent) 10%, var(--color-control));
	}

	.sidebar-action.pinned :global(svg) {
		fill: currentColor;
	}

	.scroll-sentinel {
		display: block;
		height: 1px;
		margin-top: 20px;
	}

	/* Sticky tester controls */
	.tester-controls {
		position: sticky;
		top: 0;
		z-index: var(--z-sticky);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-lg);
		padding: 10px 0;
		margin: 0 calc(clamp(20px, 3vw, 40px) * -1);
		padding-inline: clamp(20px, 3vw, 40px);
		border-bottom: 1px solid var(--color-border);
		background: color-mix(in srgb, var(--color-surface) 88%, transparent);
		backdrop-filter: blur(12px);
		transition:
			box-shadow var(--motion-standard),
			background var(--motion-standard);
	}

	.tester-controls.stuck {
		background: color-mix(in srgb, var(--color-surface) 94%, transparent);
		box-shadow: 0 8px 20px -12px rgba(0, 0, 0, 0.4);
	}

	.weight-scroller {
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
		overflow-x: auto;
		padding-bottom: 2px;
		scrollbar-width: none;
	}

	.weight-scroller::-webkit-scrollbar {
		display: none;
	}

	.weight-scroller button {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		flex: none;
		height: 32px;
		padding: 0 11px;
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		font-weight: 650;
		white-space: nowrap;
		cursor: pointer;
		transition:
			color var(--motion-fast),
			background var(--motion-fast);
	}

	.weight-scroller button small {
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
	}

	.weight-scroller button:hover {
		color: var(--color-text);
		background: var(--color-hover);
	}

	.weight-scroller button.active {
		color: var(--color-text);
		background: var(--color-selected);
	}

	.weight-scroller button.active small {
		color: var(--color-muted);
	}

	.control-cluster {
		display: flex;
		align-items: center;
		gap: var(--space-md);
		flex: none;
	}

	.size-control {
		display: inline-flex;
		align-items: center;
		gap: 9px;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-weight: 650;
	}

	.size-control input[type='range'] {
		width: 120px;
		accent-color: var(--color-accent);
	}

	.size-control output {
		min-width: 42px;
		color: var(--color-text);
		font-size: var(--text-label);
		font-variant-numeric: tabular-nums;
	}

	.control-label {
		color: var(--color-subtle);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		font-size: var(--text-micro);
	}

	.segmented {
		display: inline-flex;
		flex: none;
		padding: 2px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-control);
	}

	.segmented button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		height: 28px;
		min-width: 30px;
		padding: 0 8px;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
		transition:
			color var(--motion-fast),
			background var(--motion-fast);
	}

	.segmented button:hover {
		color: var(--color-text);
	}

	.segmented button.active {
		color: var(--color-text);
		background: var(--color-selected);
	}

	.text-segmented button {
		font-variant-numeric: tabular-nums;
	}

	/* Hero tester */
	.tester {
		padding: clamp(28px, 5vw, 56px) 0 20px;
	}

	.specimen {
		margin: 0;
		color: var(--color-text);
		line-height: 1.06;
		letter-spacing: -0.02em;
		overflow-wrap: anywhere;
		outline: none;
		cursor: text;
		transition: font-size var(--motion-standard);
	}

	.specimen:empty::before {
		content: attr(data-placeholder);
		color: var(--color-subtle);
	}

	.specimen:focus-visible {
		outline: none;
	}

	.edit-hint {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		margin-top: 18px;
		padding: 5px 10px 5px 8px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-subtle);
		background: var(--color-panel);
		font-size: var(--text-micro);
		font-weight: 600;
		cursor: text;
		transition:
			color var(--motion-fast),
			border-color var(--motion-fast);
	}

	.edit-hint:hover {
		color: var(--color-muted);
		border-color: var(--color-subtle);
	}

	/* Section scaffolding */
	.styles-section,
	.glyphs-section,
	.details-section,
	.parser-section {
		padding-top: 28px;
		border-top: 1px solid var(--color-border);
		margin-top: 8px;
	}

	.section-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-lg);
		margin-bottom: 4px;
	}

	.section-head h2,
	.details-section h2 {
		display: flex;
		align-items: baseline;
		gap: 10px;
		margin: 0;
		font-size: var(--text-heading-sm);
		font-weight: 650;
		letter-spacing: -0.015em;
	}

	.section-head .count {
		color: var(--color-subtle);
		font-size: var(--text-body-sm);
		font-variant-numeric: tabular-nums;
	}

	.section-head .count-total {
		color: var(--color-subtle);
		font-weight: 450;
	}

	.size-control.compact input[type='range'] {
		width: 96px;
	}

	/* Styles list */
	.style-list {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.style-row {
		padding: 20px 0;
		border-bottom: 1px solid var(--color-border);
		animation: style-in var(--motion-standard) both;
		animation-delay: calc(var(--row-index) * 45ms);
	}

	.style-meta {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 12px;
		margin-bottom: 12px;
	}

	.style-meta strong {
		font-size: var(--text-body-sm);
		font-weight: 650;
	}

	.style-tags {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
	}

	.tag-italic {
		font-style: italic;
		color: var(--color-muted);
	}

	.tag-format {
		padding: 1px 7px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
	}

	.style-specimen {
		margin: 0;
		color: var(--color-text);
		line-height: 1.14;
		letter-spacing: -0.015em;
		overflow-wrap: anywhere;
		transition: font-size var(--motion-fast);
	}

	/* Glyphs — catalogue on the left, live inspector on the right */
	.glyph-layout {
		display: grid;
		grid-template-columns: minmax(0, 1.55fr) minmax(380px, 0.85fr);
		gap: 32px;
		margin-top: 18px;
		align-items: start;
	}

	.glyph-catalogue {
		min-width: 0;
	}

	.glyph-accordions {
		border-top: 1px solid var(--color-border);
	}

	.glyph-accordion {
		border-bottom: 1px solid var(--color-border);
	}

	.glyph-accordion-trigger {
		display: grid;
		width: 100%;
		min-height: 64px;
		grid-template-columns: minmax(0, 1fr) auto 16px;
		gap: 14px;
		align-items: center;
		padding: 10px 12px;
		border: 0;
		border-radius: 0;
		color: var(--color-text);
		background: transparent;
		text-align: left;
		cursor: pointer;
		transition: background var(--motion-fast);
	}

	.glyph-accordion-trigger:hover {
		background: var(--color-hover);
	}

	.glyph-accordion-trigger:focus-visible {
		position: relative;
		z-index: 1;
		outline: 2px solid var(--color-focus);
		outline-offset: -3px;
	}

	.glyph-category-copy {
		display: grid;
		min-width: 0;
		gap: 3px;
	}

	.glyph-category-copy strong {
		font-size: var(--text-body-sm);
		font-weight: 650;
		letter-spacing: -0.01em;
	}

	.glyph-category-copy small {
		overflow: hidden;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-weight: 500;
		line-height: 1.35;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.glyph-category-count {
		min-width: 42px;
		padding: 3px 8px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-weight: 650;
		font-variant-numeric: tabular-nums;
		text-align: center;
	}

	.glyph-accordion-chevron {
		width: 8px;
		height: 8px;
		border-right: 1.5px solid currentColor;
		border-bottom: 1.5px solid currentColor;
		color: var(--color-muted);
		transform: rotate(45deg) translate(-1px, -1px);
		transition: transform 240ms cubic-bezier(0.16, 1, 0.3, 1);
	}

	.glyph-accordion.expanded .glyph-accordion-chevron {
		transform: rotate(225deg) translate(-1px, -1px);
	}

	.glyph-accordion-panel {
		contain: layout paint;
		padding: 0 0 16px;
	}

	.glyphs {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(54px, 1fr));
		border-top: 1px solid var(--color-border);
		border-left: 1px solid var(--color-border);
	}

	.glyphs button {
		position: relative;
		display: grid;
		aspect-ratio: 1;
		place-items: center;
		padding: 0;
		border: 0;
		border-right: 1px solid var(--color-border);
		border-bottom: 1px solid var(--color-border);
		font-size: clamp(20px, 2.4vw, 28px);
		font-family: inherit;
		color: var(--color-text);
		background: transparent;
		cursor: pointer;
		transition:
			background var(--motion-fast),
			color var(--motion-fast);
	}

	.glyphs button:hover {
		background: var(--color-hover);
	}

	.glyphs button:focus-visible {
		z-index: 1;
		outline: 2px solid var(--color-focus);
		outline-offset: -3px;
	}

	.glyphs button.active {
		color: var(--color-accent-ink);
		background: var(--color-accent);
	}

	.glyphs button.locked {
		box-shadow: inset 0 0 0 2px var(--color-focus);
	}

	.glyphs button.locked::after {
		position: absolute;
		top: 5px;
		right: 5px;
		content: 'LOCKED';
		font-family:
			Geist, 'Segoe UI Variable', 'Segoe UI', Inter, ui-sans-serif, system-ui, sans-serif;
		font-size: 7px;
		font-weight: 750;
		line-height: 1;
		letter-spacing: 0.035em;
	}

	.glyphs button.codepoint-placeholder {
		padding: 4px;
		font-family:
			Geist, 'Segoe UI Variable', 'Segoe UI', Inter, ui-sans-serif, system-ui, sans-serif;
		font-size: 9px;
		font-weight: 650;
		font-variant-numeric: tabular-nums;
		letter-spacing: -0.02em;
	}

	.glyph-batch-actions {
		display: flex;
		min-height: 48px;
		align-items: center;
		justify-content: flex-end;
		gap: 8px;
		padding: 10px 12px 0;
	}

	.glyph-batch-actions p {
		margin: 0 auto 0 0;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
	}

	.glyph-batch-actions button {
		height: 30px;
		padding: 0 10px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text);
		background: var(--color-control);
		font-family: inherit;
		font-size: var(--text-micro);
		font-weight: 650;
		cursor: pointer;
		transition: background var(--motion-fast);
	}

	.glyph-batch-actions button:hover {
		background: var(--color-hover);
	}

	.glyph-batch-actions button:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 2px;
	}

	.glyph-coverage-loading {
		display: grid;
		gap: 8px;
		padding: 8px 0;
	}

	.glyph-coverage-loading span {
		display: block;
		height: 64px;
		border-radius: var(--radius-xs);
		background: var(--color-control);
		animation: parser-pulse 1.2s ease-in-out infinite alternate;
	}

	.glyph-coverage-loading span:nth-child(2) {
		width: 92%;
	}

	.glyph-coverage-loading span:nth-child(3) {
		width: 84%;
	}

	.glyph-coverage-loading p,
	.glyph-coverage-message {
		margin: 4px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	.glyph-detail {
		position: sticky;
		top: 76px;
		align-self: start;
	}

	.glyph-detail-inner {
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-panel);
		overflow: hidden;
	}

	.glyph-view-toolbar {
		display: flex;
		min-height: 44px;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 7px 10px 7px 14px;
		border-bottom: 1px solid var(--color-border);
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-weight: 650;
	}

	.glyph-set-switcher,
	.glyph-view-switcher {
		display: inline-flex;
		align-items: center;
		padding: 2px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		background: var(--color-control);
	}

	.glyph-set-switcher button,
	.glyph-view-switcher button {
		height: 26px;
		padding: 0 9px;
		border: 0;
		border-radius: var(--radius-xs);
		color: var(--color-muted);
		background: transparent;
		font-family: inherit;
		font-size: var(--text-micro);
		font-weight: 650;
		cursor: pointer;
		transition:
			color var(--motion-fast),
			background var(--motion-fast);
	}

	.glyph-set-switcher button:hover,
	.glyph-view-switcher button:hover {
		color: var(--color-text);
	}

	.glyph-set-switcher button.active,
	.glyph-view-switcher button.active {
		color: var(--color-text);
		background: var(--color-selected);
	}

	.glyph-set-switcher button:focus-visible,
	.glyph-view-switcher button:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 2px;
	}

	.glyph-big {
		position: relative;
		display: grid;
		place-items: center;
		aspect-ratio: 1;
		padding: 20px;
		font-size: clamp(120px, 16vw, 200px);
		line-height: 1;
		color: var(--color-text);
		border-bottom: 1px solid var(--color-border);
	}

	.metric-status {
		position: absolute;
		right: 14px;
		bottom: 12px;
		left: 14px;
		color: var(--color-muted);
		font-family:
			Geist, 'Segoe UI Variable', 'Segoe UI', Inter, ui-sans-serif, system-ui, sans-serif;
		font-size: var(--text-micro);
		font-weight: 600;
		line-height: 1.35;
		text-align: center;
	}

	.metric-chart {
		position: relative;
		display: grid;
		aspect-ratio: 1;
		place-items: stretch;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-panel);
	}

	.metric-chart svg {
		display: block;
		width: 100%;
		height: 100%;
		overflow: visible;
		transition: opacity var(--motion-fast);
	}

	.metric-chart.tracing svg {
		opacity: 0.78;
	}

	.metric-glyph {
		fill: var(--color-text);
		font-optical-sizing: auto;
	}

	.metric-glyph.fallback {
		opacity: 0.22;
	}

	.glyph-outline-layer {
		animation: glyph-outline-reveal 180ms cubic-bezier(0.16, 1, 0.3, 1) both;
	}

	.glyph-outline-path {
		fill: none;
		stroke: var(--color-text);
		stroke-width: 1.35;
		stroke-linejoin: round;
		stroke-linecap: round;
		vector-effect: non-scaling-stroke;
	}

	.glyph-outline-layer.show-points .glyph-outline-path {
		stroke: color-mix(in srgb, var(--color-text) 76%, var(--color-muted));
		stroke-width: 1.05;
	}

	.glyph-handles line {
		stroke: var(--color-subtle);
		stroke-width: 0.8;
		stroke-opacity: 0.72;
		vector-effect: non-scaling-stroke;
	}

	.glyph-dimensions line,
	.glyph-dimensions rect {
		fill: none;
		stroke: var(--color-border);
		stroke-width: 0.8;
		stroke-dasharray: 3 3;
		vector-effect: non-scaling-stroke;
	}

	.glyph-outline-points circle {
		stroke: var(--color-panel);
		stroke-width: 1.5;
		vector-effect: non-scaling-stroke;
	}

	.glyph-outline-points .on-curve {
		fill: var(--color-text);
	}

	.glyph-outline-points .off-curve {
		fill: var(--color-subtle);
	}

	.glyph-outline-status {
		position: absolute;
		right: 12px;
		bottom: 10px;
		left: 12px;
		margin: 0;
		padding: 5px 8px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: color-mix(in srgb, var(--color-panel) 92%, transparent);
		font-size: var(--text-micro);
		font-weight: 600;
		line-height: 1.35;
		text-align: center;
	}

	@keyframes glyph-outline-reveal {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	.metric-guide line {
		stroke: var(--color-border);
		stroke-width: 1;
		vector-effect: non-scaling-stroke;
	}

	.metric-guide.derived line {
		stroke-dasharray: 4 3;
	}

	.metric-label,
	.metric-value {
		fill: var(--color-muted);
		stroke: var(--color-panel);
		stroke-width: 0.3em;
		stroke-linejoin: round;
		paint-order: stroke fill;
		font-family:
			Geist, 'Segoe UI Variable', 'Segoe UI', Inter, ui-sans-serif, system-ui, sans-serif;
		font-weight: 600;
	}

	.metric-value {
		font-variant-numeric: tabular-nums;
	}

	.glyph-facts {
		display: grid;
		margin: 0;
	}

	.glyph-facts div {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		padding: 11px 16px;
		font-size: var(--text-body-sm);
	}

	.glyph-facts div + div {
		border-top: 1px solid var(--color-border);
	}

	.glyph-facts dt {
		color: var(--color-muted);
	}

	.glyph-facts dd {
		margin: 0;
		font-weight: 650;
		font-variant-numeric: tabular-nums;
	}

	.glyph-facts .derived-note {
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.weight-scroller.mini {
		flex: none;
		max-width: min(60%, 420px);
	}

	.character-controls {
		display: flex;
		min-width: 0;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-sm);
	}

	.glyph-set-switcher {
		flex: none;
	}

	.face-control {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-weight: 650;
	}

	.face-control select {
		height: 30px;
		max-width: 190px;
		padding: 0 28px 0 9px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text);
		background: var(--color-control);
		font: inherit;
		font-size: var(--text-micro);
		cursor: pointer;
	}

	.face-control select:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 2px;
	}

	.weight-scroller.mini button {
		height: 28px;
		padding: 0 9px;
		font-variant-numeric: tabular-nums;
	}

	/* Details */
	.details-heading {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 20px;
	}

	.details-heading p {
		margin: 0;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
	}

	.details-grid {
		display: grid;
		grid-template-columns: minmax(360px, 1fr) minmax(440px, 1.2fr);
		gap: 32px;
		margin-top: 16px;
	}

	.parsed-summary {
		min-width: 0;
	}

	.summary {
		display: grid;
		gap: 0;
		margin: 0;
		align-content: start;
	}

	.summary div {
		display: flex;
		justify-content: space-between;
		gap: 18px;
		padding: 10px 0;
		border-bottom: 1px solid var(--color-border);
		font-size: var(--text-body-sm);
	}

	.summary dt {
		color: var(--color-muted);
	}

	.summary dd {
		margin: 0;
		font-weight: 650;
		text-align: right;
		overflow-wrap: anywhere;
	}

	.metric-summary {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		margin: 20px 0 0;
		border-block: 1px solid var(--color-border);
	}

	.metric-summary div {
		min-width: 0;
		padding: 10px 0;
	}

	.metric-summary div:not(:nth-child(3n + 1)) {
		padding-left: 14px;
		border-left: 1px solid var(--color-border);
	}

	.metric-summary div:nth-child(n + 4) {
		border-top: 1px solid var(--color-border);
	}

	.metric-summary dt {
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.metric-summary dd {
		margin: 3px 0 0;
		font-size: var(--text-body-sm);
		font-weight: 650;
		font-variant-numeric: tabular-nums;
	}

	.axis-summary {
		margin-top: 20px;
	}

	.axis-summary h3 {
		margin: 0 0 9px;
		color: var(--color-muted);
		font-size: var(--text-label);
		font-weight: 650;
	}

	.axis-summary ul {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(132px, 1fr));
		gap: 1px;
		margin: 0;
		padding: 1px;
		background: var(--color-border);
		list-style: none;
	}

	.axis-summary li {
		display: grid;
		gap: 2px;
		padding: 9px 10px;
		background: var(--color-surface);
		font-variant-numeric: tabular-nums;
	}

	.axis-summary strong {
		font-family: ui-monospace, 'Cascadia Code', monospace;
		font-size: var(--text-label);
	}

	.axis-summary span,
	.axis-summary small {
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.credits-summary {
		display: grid;
		gap: 6px;
		margin: 18px 0 0;
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.credits-summary div {
		display: grid;
		grid-template-columns: 88px minmax(0, 1fr);
		gap: 12px;
	}

	.credits-summary dd {
		margin: 0;
		color: var(--color-text);
		overflow-wrap: anywhere;
	}

	.license-copy {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
	}

	.details-loading {
		display: grid;
		gap: 9px;
		padding: 4px 0 16px;
	}

	.details-loading span {
		display: block;
		height: 34px;
		border-radius: var(--radius-xs);
		background: var(--color-control);
		animation: parser-pulse 1.2s ease-in-out infinite alternate;
	}

	.details-loading span:nth-child(2) {
		width: 88%;
	}

	.details-loading span:nth-child(3) {
		width: 94%;
	}

	.details-loading span:nth-child(4) {
		width: 76%;
	}

	.details-loading p,
	.details-error {
		margin: 4px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	.faces-table {
		font-size: var(--text-body-sm);
		font-variant-numeric: tabular-nums;
	}

	.faces-head,
	.faces-row {
		display: grid;
		grid-template-columns: minmax(120px, 1fr) 70px minmax(0, 1.4fr);
		gap: 16px;
		align-items: center;
		padding: 10px 0;
		border-bottom: 1px solid var(--color-border);
	}

	.faces-head {
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-weight: 650;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.faces-row.selected {
		margin-inline: -8px;
		padding-inline: 8px;
		border-radius: var(--radius-xs);
		background: var(--color-selected);
	}

	.face-style em {
		margin-left: 6px;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-style: italic;
	}

	.selected-face-label {
		margin-left: 7px;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-weight: 600;
	}

	.face-weight {
		color: var(--color-muted);
	}

	.face-file {
		overflow: hidden;
		color: var(--color-muted);
		font-size: var(--text-micro);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* Parser snapshot — exhaustive data stays disclosed and lazy. */
	.parser-section details {
		border-bottom: 1px solid var(--color-border);
	}

	.parser-section summary {
		display: flex;
		min-height: 58px;
		align-items: center;
		justify-content: space-between;
		gap: 20px;
		padding: 0 2px;
		cursor: pointer;
		list-style: none;
	}

	.parser-section summary::-webkit-details-marker {
		display: none;
	}

	.parser-section summary:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 3px;
	}

	.parser-heading {
		display: grid;
		gap: 3px;
	}

	.parser-heading strong {
		font-size: var(--text-heading-sm);
		font-weight: 650;
		letter-spacing: -0.015em;
	}

	.parser-heading small,
	.parser-disclosure {
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.parser-disclosure {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		font-weight: 650;
	}

	.parser-disclosure::after {
		content: '';
		width: 7px;
		height: 7px;
		border-right: 1.5px solid currentColor;
		border-bottom: 1.5px solid currentColor;
		transform: rotate(45deg) translateY(-2px);
		transition: transform var(--motion-fast);
	}

	.parser-section details[open] .parser-disclosure::after {
		transform: rotate(225deg) translate(-2px, -2px);
	}

	.parser-content {
		padding: 8px 0 24px;
	}

	.parser-toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 10px;
	}

	.parser-toolbar p,
	.parser-loading p,
	.parser-error {
		margin: 0;
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.parser-toolbar button,
	.load-parser-action {
		display: inline-flex;
		min-height: 34px;
		align-items: center;
		justify-content: center;
		padding: 0 11px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.parser-toolbar button:hover,
	.load-parser-action:hover {
		background: var(--color-selected);
	}

	.parser-json {
		max-height: min(64vh, 680px);
		overflow: auto;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-bg);
	}

	.parser-json:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 2px;
	}

	.parser-json pre {
		margin: 0;
		padding: 16px;
		font-family: 'Cascadia Code', 'SFMono-Regular', Consolas, monospace;
		font-size: 11px;
		line-height: 1.55;
		tab-size: 2;
		white-space: pre;
	}

	.parser-loading {
		display: grid;
		gap: 8px;
	}

	.parser-loading span {
		display: block;
		height: 8px;
		border-radius: var(--radius-xs);
		background: var(--color-skeleton);
		animation: parser-pulse 1.2s ease-in-out infinite alternate;
	}

	.parser-loading span:nth-child(1) {
		width: 72%;
	}

	.parser-loading span:nth-child(2) {
		width: 54%;
	}

	.parser-loading span:nth-child(3) {
		width: 63%;
	}

	.parser-error {
		color: var(--color-danger);
	}

	/* Missing state */
	.missing-preview {
		max-width: 420px;
		margin: 12vh auto 0;
		text-align: center;
	}

	.missing-icon {
		display: grid;
		width: 52px;
		height: 52px;
		margin: 0 auto 18px;
		place-items: center;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-muted);
		background: var(--color-panel);
	}

	.missing-preview p {
		margin: 7px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	.missing-preview button {
		margin-top: 20px;
		padding: 0 12px;
	}

	@keyframes style-in {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes parser-pulse {
		to {
			opacity: 0.42;
		}
	}

	/* Responsive */
	@media (max-width: 900px) {
		.tester-controls {
			flex-direction: column;
			align-items: stretch;
			gap: var(--space-sm);
		}

		.control-cluster {
			justify-content: space-between;
			flex-wrap: wrap;
			gap: var(--space-sm);
		}

		.details-grid {
			grid-template-columns: 1fr;
			gap: 24px;
		}

		.glyph-layout {
			grid-template-columns: 1fr;
			gap: 20px;
		}

		.glyph-detail {
			position: static;
			order: -1;
		}

		.glyph-big {
			aspect-ratio: auto;
			min-height: 180px;
		}

		.metric-chart {
			aspect-ratio: auto;
			height: min(72vw, 360px);
		}
	}

	@media (max-width: 600px) {
		.page-inner {
			padding: 18px 16px 40px;
		}

		.tester-controls {
			margin-inline: -16px;
			padding-inline: 16px;
		}

		.heading-row {
			align-items: flex-start;
			flex-direction: column;
			gap: 16px;
		}

		.sidebar-action {
			width: 100%;
			justify-content: center;
		}

		.size-control input[type='range'] {
			width: 100%;
			flex: 1;
		}

		.size-control {
			flex: 1;
		}

		.section-head {
			align-items: flex-start;
			flex-direction: column;
		}

		.character-controls {
			width: 100%;
			justify-content: space-between;
			flex-wrap: wrap;
		}

		.weight-scroller.mini {
			max-width: 55%;
		}

		.glyph-accordion-trigger {
			grid-template-columns: minmax(0, 1fr) auto 14px;
			gap: 10px;
			padding-inline: 8px;
		}

		.glyph-category-copy small {
			display: -webkit-box;
			white-space: normal;
			-webkit-box-orient: vertical;
			-webkit-line-clamp: 2;
			line-clamp: 2;
		}

		.glyph-accordion-panel {
			padding-inline: 0;
		}

		.glyph-batch-actions {
			flex-wrap: wrap;
			padding-inline: 8px;
		}

		.glyph-batch-actions p {
			width: 100%;
		}

		.faces-head,
		.faces-row {
			grid-template-columns: 1fr 56px;
		}

		.face-file {
			display: none;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.style-row {
			animation: none;
		}

		.specimen,
		.style-specimen,
		.parser-disclosure::after,
		.glyph-accordion-chevron,
		.glyph-accordion-trigger {
			transition: none;
		}

		.glyph-outline-layer {
			animation: none;
		}

		.glyph-set-switcher button,
		.glyph-view-switcher button,
		.metric-chart svg {
			transition: none;
		}

		.parser-loading span,
		.details-loading span,
		.glyph-coverage-loading span {
			animation: none;
		}
	}
</style>
