<script lang="ts">
	import type { ValidatedLocalFont } from '$lib/bindings/ValidatedLocalFont';

	import Icon from './Icon.svelte';

	let {
		font,
		previewText,
		onClose
	}: {
		font: ValidatedLocalFont;
		previewText: string;
		onClose: () => void;
	} = $props();

	let closeButton = $state<HTMLButtonElement>();

	$effect(() => {
		closeButton?.focus();
	});

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.stopPropagation();
			onClose();
		}
	}

	// The face is registered on the document under its synthetic family name, so the
	// specimen renders the exact validated bytes and no installed family can shadow it.
	let specimenStyle = $derived(`font-family: "${font.previewFamily}", system-ui, sans-serif;`);
	let title = $derived(font.faces[0]?.familyName ?? font.fileName);
	let sample = $derived(previewText.trim() || title);
</script>

<svelte:window onkeydown={handleKeydown} />

<div
	class="overlay"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget) onClose();
	}}
>
	<div class="panel" role="dialog" aria-modal="true" aria-labelledby="local-preview-title">
		<header class="panel-head">
			<div class="lead">
				<p class="eyebrow">Local preview, not installed</p>
				<h2 id="local-preview-title">{title}</h2>
				<p class="meta">
					{font.format} · {font.faceCount}
					{font.faceCount === 1 ? 'face' : 'faces'} · {font.fileName}
				</p>
			</div>
			<button
				bind:this={closeButton}
				type="button"
				class="close"
				aria-label="Close local font preview"
				onclick={onClose}
			>
				<Icon name="close" size={16} />
			</button>
		</header>

		<p class="specimen" style={specimenStyle}>{sample}</p>

		<ul class="faces">
			{#each font.faces as face (face.faceIndex)}
				<li>
					<span class="face-name">{face.fullName}</span>
					<span class="face-detail">
						{face.subfamilyName}
						{#if face.isVariable}<span class="badge">Variable</span>{/if}
						<span class="glyphs">{face.glyphCount.toLocaleString()} glyphs</span>
					</span>
				</li>
			{/each}
		</ul>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: var(--z-modal);
		display: grid;
		place-items: center;
		padding: var(--space-2xl);
		background: color-mix(in srgb, var(--color-bg) 68%, transparent);
		-webkit-backdrop-filter: blur(6px);
		backdrop-filter: blur(6px);
	}

	.panel {
		display: flex;
		width: min(640px, 100%);
		max-height: 100%;
		flex-direction: column;
		gap: var(--space-xl);
		overflow-y: auto;
		padding: var(--space-2xl);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-surface);
		box-shadow: var(--shadow-floating);
	}

	.panel-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-lg);
	}

	.eyebrow {
		margin: 0 0 4px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-weight: 650;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	h2 {
		margin: 0;
		font-size: var(--text-heading-sm);
		font-weight: 650;
	}

	.meta {
		margin: 6px 0 0;
		color: var(--color-muted);
		font-size: var(--text-label);
	}

	.close {
		display: grid;
		width: 32px;
		height: 32px;
		flex: none;
		place-items: center;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-muted);
		background: var(--color-control);
		cursor: pointer;
		transition:
			color var(--motion-fast),
			background var(--motion-fast);
	}

	.close:hover {
		color: var(--color-text);
		background: var(--color-hover);
	}

	.specimen {
		margin: 0;
		color: var(--color-text);
		font-size: clamp(2rem, 6vw, 3.5rem);
		line-height: 1.1;
		overflow-wrap: anywhere;
	}

	.faces {
		display: flex;
		margin: 0;
		padding: 0;
		flex-direction: column;
		gap: 2px;
		list-style: none;
	}

	.faces li {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-lg);
		padding: 8px 10px;
		border-radius: var(--radius-sm);
		background: var(--color-panel);
	}

	.face-name {
		min-width: 0;
		overflow: hidden;
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.face-detail {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: var(--space-sm);
		color: var(--color-muted);
		font-size: var(--text-label);
	}

	.badge {
		padding: 1px 7px;
		border-radius: var(--radius-shell);
		color: var(--color-accent-ink);
		background: var(--color-accent);
		font-size: var(--text-micro);
		font-weight: 650;
	}

	.glyphs {
		font-variant-numeric: tabular-nums;
	}
</style>
