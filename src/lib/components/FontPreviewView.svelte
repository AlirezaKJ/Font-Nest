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

	// Character sets for the glyph specimen — rendered in the family itself.
	const GLYPH_GROUPS: { label: string; glyphs: string[] }[] = [
		{
			label: 'Uppercase',
			glyphs: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('')
		},
		{
			label: 'Lowercase',
			glyphs: 'abcdefghijklmnopqrstuvwxyz'.split('')
		},
		{
			label: 'Figures',
			glyphs: '0123456789'.split('')
		},
		{
			label: 'Symbols',
			glyphs: '& @ # % $ € £ ¥ * § ¶ † ‡ © ® ™ → ↑ ↓ ← ½ ¼ ¾ ± × ÷ = < > ~ ^ / \\ | { } [ ] ( )'.split(
				' '
			)
		},
		{
			label: 'Punctuation',
			glyphs: '. , : ; ! ? ‽ · • … “ ” ‘ ’ " \' « » ‹ › — – - _'.split(' ')
		},
		{
			label: 'Accented',
			glyphs: 'À Á Â Ã Ä Å Æ Ç È É Ê Ë Ñ Ø Œ ß à á â ä å æ ç é ê ë ñ ø œ ÿ'.split(' ')
		}
	];

	type Alignment = 'left' | 'center';
	type LetterCase = 'as-typed' | 'upper' | 'lower';
</script>

<script lang="ts">
	import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';

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
	let selectedGlyph = $state('A');

	let specimenEl = $state<HTMLElement>();
	let sentinelEl = $state<HTMLElement>();
	let stuck = $state(false);

	function safeFontStack(name: string): string {
		return `"${name.replace(/["\\;\n\r]/g, '')}", system-ui, sans-serif`;
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
	let selectedCodepoint = $derived(
		'U+' + (selectedGlyph.codePointAt(0) ?? 0).toString(16).toUpperCase().padStart(4, '0')
	);
	let displayText = $derived(previewText.trim() || family?.name || 'Type to preview');
	let caseTransform = $derived(
		letterCase === 'upper' ? 'uppercase' : letterCase === 'lower' ? 'lowercase' : 'none'
	);

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
		if (specimenEl && document.activeElement !== specimenEl && specimenEl.textContent !== text) {
			specimenEl.textContent = text;
		}
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
		glyphWeightOverride = weight;
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
									{#if face.style === 'italic'}<span class="tag-italic">Italic</span>{/if}
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
					<h2 id="glyphs-title">Characters</h2>
					<div class="weight-scroller mini" role="group" aria-label="Character weight">
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
				</div>

				<div class="glyph-layout">
					<div class="glyph-catalogue">
						{#each GLYPH_GROUPS as group (group.label)}
							<div class="glyph-group">
								<h3>{group.label}</h3>
								<div
									class="glyphs"
									style={`font-family: ${safeFontStack(family.name)}; font-weight: ${glyphWeight};`}
								>
									{#each group.glyphs as glyph, i (group.label + i)}
										<button
											type="button"
											class:active={selectedGlyph === glyph}
											aria-pressed={selectedGlyph === glyph}
											aria-label={`Inspect ${glyph}`}
											onclick={() => (selectedGlyph = glyph)}
											onmouseenter={() => (selectedGlyph = glyph)}
											onfocus={() => (selectedGlyph = glyph)}
										>
											{glyph}
										</button>
									{/each}
								</div>
							</div>
						{/each}
					</div>

					<aside class="glyph-detail" aria-live="polite">
						<div class="glyph-detail-inner">
							<div
								class="glyph-big"
								style={`font-family: ${safeFontStack(family.name)}; font-weight: ${glyphWeight};`}
							>
								{selectedGlyph}
							</div>
							<dl class="glyph-facts">
								<div><dt>Unicode</dt><dd>{selectedCodepoint}</dd></div>
								<div><dt>Weight</dt><dd>{glyphWeight}</dd></div>
							</dl>
						</div>
					</aside>
				</div>
			</section>

			<section class="details-section" aria-labelledby="details-title">
				<h2 id="details-title">Family details</h2>
				<div class="details-grid">
					<dl class="summary">
						<div><dt>Faces</dt><dd>{family.faceCount}</dd></div>
						<div><dt>Files</dt><dd>{family.fileCount}</dd></div>
						<div><dt>Weights</dt><dd>{availableWeights.join(', ')}</dd></div>
						<div><dt>Formats</dt><dd>{family.formats.join(', ')}</dd></div>
						<div><dt>Sources</dt><dd>{family.sources.join(', ')}</dd></div>
						<div>
							<dt>Spacing</dt>
							<dd>{family.monospaced ? 'Monospaced' : 'Proportional'}</dd>
						</div>
					</dl>

					<div class="faces-table" role="table" aria-label="Installed faces">
						<div class="faces-head" role="row">
							<span role="columnheader">Style</span>
							<span role="columnheader">Weight</span>
							<span role="columnheader">File</span>
						</div>
						{#each family.faces as face (face.id)}
							<div class="faces-row" role="row">
								<span role="cell" class="face-style">
									{face.styleName || weightName(face.weight)}
									{#if face.style === 'italic'}<em>Italic</em>{/if}
								</span>
								<span role="cell" class="face-weight">{face.weight}</span>
								<span role="cell" class="face-file" title={face.fileName}>{face.fileName}</span>
							</div>
						{/each}
					</div>
				</div>
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
		border-top: 1px solid var(--color-border);
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
	.details-section {
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
		grid-template-columns: minmax(0, 1fr) minmax(240px, 320px);
		gap: 32px;
		margin-top: 18px;
		align-items: start;
	}

	.glyph-catalogue {
		display: grid;
		gap: 22px;
		min-width: 0;
	}

	.glyph-group h3 {
		margin: 0 0 10px;
		color: var(--color-muted);
		font-size: var(--text-label);
		font-weight: 650;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.glyphs {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(54px, 1fr));
		border-top: 1px solid var(--color-border);
		border-left: 1px solid var(--color-border);
	}

	.glyphs button {
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

	.glyphs button.active {
		color: var(--color-accent-ink);
		background: var(--color-accent);
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

	.glyph-big {
		display: grid;
		place-items: center;
		aspect-ratio: 1;
		padding: 20px;
		font-size: clamp(120px, 16vw, 200px);
		line-height: 1;
		color: var(--color-text);
		border-bottom: 1px solid var(--color-border);
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

	.weight-scroller.mini {
		flex: none;
		max-width: min(60%, 420px);
	}

	.weight-scroller.mini button {
		height: 28px;
		padding: 0 9px;
		font-variant-numeric: tabular-nums;
	}

	/* Details */
	.details-grid {
		display: grid;
		grid-template-columns: minmax(220px, 0.6fr) minmax(0, 1.4fr);
		gap: 32px;
		margin-top: 16px;
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

	.face-style em {
		margin-left: 6px;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-style: italic;
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
		.style-specimen {
			transition: none;
		}
	}
</style>
