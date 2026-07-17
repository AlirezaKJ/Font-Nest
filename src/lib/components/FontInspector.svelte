<script lang="ts">
	import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';

	let {
		family,
		previewText,
		previewSize,
		previewWeight,
		onPreviewText,
		onPreviewSize,
		onPreviewWeight
	}: {
		family: FontFamilySummary | null;
		previewText: string;
		previewSize: number;
		previewWeight: number;
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
	let displayedFaces = $derived(family?.faces.slice(0, 6) ?? []);
</script>

<aside class="inspector" aria-label="Font inspector">
	{#if family}
		<header>
			<div>
				<p class="section-label">Inspector</p>
				<h2>{family.name}</h2>
			</div>
			{#if family.hasConflict}
				<span class="state-chip warning">Potential conflict</span>
			{:else}
				<span class="state-chip">{family.sources.join(' · ')}</span>
			{/if}
		</header>

		<div class="specimen" style={specimenStyle} aria-label={`${family.name} specimen`}>Ag</div>

		<label class="preview-copy-label" for="preview-copy">Preview text</label>
		<textarea
			id="preview-copy"
			rows="2"
			value={previewText}
			oninput={(event) => onPreviewText(event.currentTarget.value)}></textarea>

		<section class="detail-group" aria-labelledby="preview-controls-title">
			<h3 id="preview-controls-title">Preview controls</h3>
			<label class="control-label" for="preview-weight">
				<span>Weight</span><strong>{previewWeight}</strong>
			</label>
			<select
				id="preview-weight"
				value={previewWeight}
				onchange={(event) => onPreviewWeight(Number(event.currentTarget.value))}
			>
				{#each availableWeights as weight (weight)}
					<option value={weight}>{weight}</option>
				{/each}
			</select>

			<label class="control-label size-label" for="preview-size">
				<span>Size</span><strong>{previewSize}px</strong>
			</label>
			<input
				id="preview-size"
				type="range"
				min="24"
				max="96"
				step="1"
				value={previewSize}
				oninput={(event) => onPreviewSize(Number(event.currentTarget.value))}
			/>
		</section>

		<section class="detail-group" aria-labelledby="family-details-title">
			<h3 id="family-details-title">Family details</h3>
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

		<section class="detail-group" aria-labelledby="faces-title">
			<h3 id="faces-title">Available faces</h3>
			<ul class="face-list">
				{#each displayedFaces as face (face.id)}
					<li>
						<span>{face.styleName}</span>
						<small>{face.fileName}</small>
					</li>
				{/each}
			</ul>
			{#if family.faces.length > displayedFaces.length}
				<p class="more-faces">+{family.faces.length - displayedFaces.length} more faces</p>
			{/if}
		</section>

		<section class="glyph-group" aria-labelledby="glyph-sample-title">
			<h3 id="glyph-sample-title">Character sample</h3>
			<div class="glyphs" style={`font-family: ${safeFontStack(family.name)}`}>
				{#each glyphs as glyph (glyph)}<span>{glyph}</span>{/each}
			</div>
		</section>
	{:else}
		<div class="inspector-empty">
			<p class="section-label">Inspector</p>
			<h2>Select a font family</h2>
			<p>
				Choose a family in the catalogue to inspect its styles, files, and preview controls.
			</p>
		</div>
	{/if}
</aside>

<style>
	.inspector {
		position: sticky;
		top: 0;
		height: var(--app-content-height, 100dvh);
		min-width: 0;
		align-self: start;
		overflow-y: auto;
		padding: 20px;
		border-left: 1px solid var(--color-border);
		background: var(--color-raised);
	}

	header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 12px;
	}

	.section-label {
		margin: 0;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-weight: 650;
		letter-spacing: 0.045em;
		text-transform: uppercase;
	}

	h2 {
		margin: 5px 0 0;
		font-size: var(--text-heading-sm);
		line-height: 1.2;
		letter-spacing: -0.025em;
		text-wrap: balance;
	}

	.state-chip {
		display: inline-flex;
		min-height: 24px;
		align-items: center;
		padding: 0 8px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-muted);
		background: var(--color-panel);
		font-size: var(--text-micro);
		font-weight: 600;
		white-space: nowrap;
	}

	.state-chip.warning {
		color: var(--color-warning);
	}

	.specimen {
		min-height: 112px;
		margin-top: 18px;
		padding: 18px 0 16px;
		border-top: 1px solid var(--color-border);
		border-bottom: 1px solid var(--color-border);
		line-height: 0.95;
		letter-spacing: -0.04em;
		word-break: break-word;
	}

	.preview-copy-label,
	.control-label {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-weight: 600;
	}

	.preview-copy-label {
		margin-top: 14px;
	}

	textarea,
	select {
		width: 100%;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
	}

	textarea {
		min-height: 62px;
		margin-top: 7px;
		padding: 9px 10px;
		font: inherit;
		font-size: var(--text-body-sm);
		line-height: 1.45;
		resize: vertical;
	}

	select {
		height: 36px;
		margin-top: 7px;
		padding: 0 10px;
		font-size: var(--text-body-sm);
	}

	.detail-group,
	.glyph-group {
		padding: 16px 0;
		border-bottom: 1px solid var(--color-border);
	}

	.glyph-group {
		border-bottom: 0;
	}

	h3 {
		margin: 0 0 11px;
		font-size: var(--text-label);
		font-weight: 650;
	}

	.control-label strong {
		color: var(--color-text);
		font-variant-numeric: tabular-nums;
	}

	.size-label {
		margin-top: 14px;
	}

	input[type='range'] {
		width: 100%;
		margin: 9px 0 0;
		accent-color: var(--color-accent);
	}

	dl {
		display: grid;
		gap: 8px;
		margin: 0;
	}

	dl div {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 16px;
		font-size: var(--text-micro);
	}

	dt {
		color: var(--color-muted);
	}

	dd {
		margin: 0;
		font-weight: 600;
		text-align: right;
	}

	.face-list {
		display: grid;
		gap: 9px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.face-list li {
		display: grid;
		gap: 2px;
		font-size: var(--text-body-sm);
	}

	.face-list small,
	.more-faces {
		overflow: hidden;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.more-faces {
		margin: 10px 0 0;
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

	.inspector-empty {
		max-width: 32ch;
		padding-top: 8px;
	}

	.inspector-empty p:last-child {
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.55;
	}

	@media (max-width: 1119px) {
		.inspector {
			position: static;
			height: auto;
			align-self: auto;
			overflow-y: visible;
			border-top: 1px solid var(--color-border);
			border-left: 0;
		}

		.inspector:not(:has(.inspector-empty)) {
			display: grid;
			grid-template-columns: minmax(220px, 0.9fr) minmax(260px, 1.1fr);
			align-content: start;
			gap: 0 24px;
		}

		header,
		.specimen,
		.preview-copy-label,
		textarea {
			grid-column: 1;
		}

		.detail-group,
		.glyph-group {
			grid-column: 2;
		}

		.detail-group:first-of-type {
			grid-row: 1 / span 2;
		}
	}

	@media (max-width: 700px) {
		.inspector:not(:has(.inspector-empty)) {
			display: block;
		}
	}
</style>
