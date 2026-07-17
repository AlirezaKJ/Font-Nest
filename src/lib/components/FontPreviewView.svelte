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

	const glyphs = ['A', 'g', 'ß', 'Æ', 'ø', 'Ж', '7', '&', '@', '?', '½', '→'];

	function safeFontStack(name: string): string {
		return `"${name.replace(/["\\;\n\r]/g, '')}", system-ui, sans-serif`;
	}

	let specimenStyle = $derived(
		family
			? `font-family: ${safeFontStack(family.name)}; font-size: ${previewSize}px; font-weight: ${previewWeight};`
			: ''
	);
	let availableWeights = $derived(family?.weights.length ? family.weights : [400]);
</script>

<section class="font-preview-page" aria-labelledby="font-preview-title">
	{#if family}
		<header class="preview-header">
			<button type="button" class="back-action" onclick={onBack}>
				<span class="back-icon"><Icon name="chevron" size={15} /></span>
				Library
			</button>

			<div class="heading-row">
				<div class="heading-copy">
					<h1 id="font-preview-title">{family.name}</h1>
					<p>
						{family.sources.join(' · ')} · {family.formats.join(' · ')} · {family.faceCount}
						{family.faceCount === 1 ? 'face' : 'faces'}
					</p>
				</div>
				<button
					type="button"
					class:pinned
					class="sidebar-action"
					aria-pressed={pinned}
					onclick={onTogglePinned}
				>
					<Icon name="bookmark" size={16} />
					<span>{pinned ? 'Remove from sidebar' : 'Add to sidebar'}</span>
				</button>
			</div>
		</header>

		<div class="specimen" style={specimenStyle} aria-label={`${family.name} preview`}>
			{previewText || family.name}
		</div>

		<section class="preview-controls" aria-labelledby="preview-controls-title">
			<h2 id="preview-controls-title">Preview controls</h2>
			<label class="copy-control" for="page-preview-copy">
				<span>Preview text</span>
				<textarea
					id="page-preview-copy"
					rows="2"
					value={previewText}
					oninput={(event) => onPreviewText(event.currentTarget.value)}></textarea>
			</label>

			<label for="page-preview-weight">
				<span>Weight</span>
				<select
					id="page-preview-weight"
					value={previewWeight}
					onchange={(event) => onPreviewWeight(Number(event.currentTarget.value))}
				>
					{#each availableWeights as weight (weight)}
						<option value={weight}>{weight}</option>
					{/each}
				</select>
			</label>

			<label class="size-control" for="page-preview-size">
				<span>Size <strong>{previewSize}px</strong></span>
				<input
					id="page-preview-size"
					type="range"
					min="24"
					max="96"
					step="1"
					value={previewSize}
					oninput={(event) => onPreviewSize(Number(event.currentTarget.value))}
				/>
			</label>
		</section>

		<div class="preview-information">
			<section class="family-summary" aria-labelledby="family-summary-title">
				<h2 id="family-summary-title">Family details</h2>
				<dl>
					<div>
						<dt>Faces</dt>
						<dd>{family.faceCount}</dd>
					</div>
					<div>
						<dt>Files</dt>
						<dd>{family.fileCount}</dd>
					</div>
					<div>
						<dt>Format</dt>
						<dd>{family.formats.join(', ')}</dd>
					</div>
					<div>
						<dt>Spacing</dt>
						<dd>{family.monospaced ? 'Monospaced' : 'Proportional'}</dd>
					</div>
				</dl>
			</section>

			<section class="faces" aria-labelledby="available-faces-title">
				<h2 id="available-faces-title">Available faces</h2>
				<ul>
					{#each family.faces as face (face.id)}
						<li>
							<span>{face.styleName}</span>
							<small>{face.weight} · {face.fileName}</small>
						</li>
					{/each}
				</ul>
			</section>

			<section class="glyph-sample" aria-labelledby="glyph-sample-title">
				<h2 id="glyph-sample-title">Character sample</h2>
				<div class="glyphs" style={`font-family: ${safeFontStack(family.name)}`}>
					{#each glyphs as glyph (glyph)}<span>{glyph}</span>{/each}
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
		padding: 24px clamp(20px, 3vw, 40px) 40px;
		background: var(--color-surface);
	}

	.preview-header {
		max-width: 1180px;
		margin: 0 auto;
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
		margin-top: 18px;
	}

	.heading-copy {
		min-width: 0;
	}

	h1 {
		margin: 0;
		font-size: 2rem;
		line-height: 1.1;
		letter-spacing: -0.035em;
		text-wrap: balance;
	}

	.heading-copy p,
	.missing-preview p {
		margin: 7px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	.sidebar-action {
		flex: none;
		padding: 0 12px;
	}

	.sidebar-action.pinned {
		background: var(--color-selected);
	}

	.sidebar-action.pinned :global(svg) {
		fill: currentColor;
	}

	.specimen {
		display: flex;
		max-width: 1180px;
		min-height: 290px;
		align-items: center;
		margin: 32px auto 0;
		overflow-wrap: anywhere;
		padding: 32px 0;
		border-top: 1px solid var(--color-border);
		border-bottom: 1px solid var(--color-border);
		line-height: 1.04;
		letter-spacing: -0.035em;
	}

	.preview-controls,
	.preview-information {
		max-width: 1180px;
		margin-right: auto;
		margin-left: auto;
	}

	.preview-controls {
		display: grid;
		grid-template-columns: minmax(260px, 1fr) 140px minmax(220px, 0.65fr);
		align-items: end;
		gap: 18px;
		padding: 24px 0;
		border-bottom: 1px solid var(--color-border);
	}

	.preview-controls h2 {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}

	.preview-controls label {
		display: grid;
		gap: 7px;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-weight: 650;
	}

	.preview-controls label > span {
		display: flex;
		justify-content: space-between;
		gap: 12px;
	}

	.preview-controls strong {
		color: var(--color-text);
		font-variant-numeric: tabular-nums;
	}

	textarea,
	select {
		width: 100%;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font: inherit;
	}

	textarea {
		min-height: 62px;
		padding: 9px 10px;
		font-size: var(--text-body-sm);
		line-height: 1.45;
		resize: vertical;
	}

	select {
		height: 36px;
		padding: 0 10px;
		font-size: var(--text-body-sm);
	}

	input[type='range'] {
		width: 100%;
		margin: 8px 0;
		accent-color: var(--color-accent);
	}

	.preview-information {
		display: grid;
		grid-template-columns: minmax(220px, 0.7fr) minmax(300px, 1.3fr);
		gap: 0 32px;
		padding-top: 8px;
	}

	.preview-information section {
		padding: 24px 0;
		border-bottom: 1px solid var(--color-border);
	}

	.preview-information h2 {
		margin: 0 0 14px;
		font-size: var(--text-title);
		letter-spacing: -0.015em;
	}

	.family-summary dl {
		display: grid;
		gap: 10px;
		margin: 0;
	}

	.family-summary dl div {
		display: flex;
		justify-content: space-between;
		gap: 18px;
		font-size: var(--text-body-sm);
	}

	dt {
		color: var(--color-muted);
	}

	dd {
		margin: 0;
		font-weight: 650;
		text-align: right;
	}

	.faces {
		grid-row: span 2;
	}

	.faces ul {
		display: grid;
		gap: 0;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.faces li {
		display: grid;
		grid-template-columns: minmax(110px, 0.45fr) minmax(0, 1fr);
		gap: 16px;
		padding: 9px 0;
		border-top: 1px solid var(--color-border);
		font-size: var(--text-body-sm);
	}

	.faces small {
		overflow: hidden;
		color: var(--color-muted);
		font-size: var(--text-micro);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.glyphs {
		display: grid;
		grid-template-columns: repeat(6, 1fr);
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

	.missing-preview button {
		margin-top: 20px;
		padding: 0 12px;
	}

	@media (max-width: 860px) {
		.preview-controls {
			grid-template-columns: minmax(240px, 1fr) 140px;
		}

		.size-control {
			grid-column: 1 / -1;
		}

		.preview-information {
			grid-template-columns: 1fr;
		}

		.faces {
			grid-row: auto;
		}
	}

	@media (max-width: 600px) {
		.font-preview-page {
			padding: 18px 16px 32px;
		}

		.heading-row {
			align-items: flex-start;
			flex-direction: column;
		}

		.sidebar-action {
			width: 100%;
			justify-content: center;
		}

		.specimen {
			min-height: 220px;
			margin-top: 24px;
			padding: 24px 0;
		}

		.preview-controls {
			grid-template-columns: 1fr;
		}

		.size-control {
			grid-column: auto;
		}

		.faces li {
			grid-template-columns: 1fr;
			gap: 3px;
		}
	}
</style>
