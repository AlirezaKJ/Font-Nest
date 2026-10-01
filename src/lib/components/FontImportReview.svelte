<script lang="ts">
	import type { ImportOutcome } from '$lib/bindings/ImportOutcome';
	import type { ImportPlan } from '$lib/bindings/ImportPlan';
	import {
		embeddingLabel,
		importableCount,
		outcomeSummary,
		planSummary,
		verdictDetail,
		verdictLabel,
		verdictTone
	} from '$lib/fonts/import';

	import Icon from './Icon.svelte';

	let {
		plan,
		outcomes,
		importing,
		onConfirm,
		onClose
	}: {
		plan: ImportPlan;
		/** Null until an import has run; the same panel then reports what happened. */
		outcomes: ImportOutcome[] | null;
		importing: boolean;
		onConfirm: () => void;
		onClose: () => void;
	} = $props();

	let closeButton = $state<HTMLButtonElement>();

	$effect(() => {
		closeButton?.focus();
	});

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && !importing) {
			event.stopPropagation();
			onClose();
		}
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	let ready = $derived(importableCount(plan));
</script>

<svelte:window onkeydown={handleKeydown} />

<div
	class="overlay"
	role="presentation"
	onclick={(event) => {
		if (event.target === event.currentTarget && !importing) onClose();
	}}
>
	<div class="panel" role="dialog" aria-modal="true" aria-labelledby="font-import-title">
		<header class="panel-head">
			<div class="lead">
				<p class="eyebrow">
					{outcomes ? 'Imported' : 'Nothing has been installed yet'}
				</p>
				<h2 id="font-import-title">
					{outcomes ? 'Import finished' : 'Review these fonts'}
				</h2>
				<p class="meta">{outcomes ? outcomeSummary(outcomes) : planSummary(plan)}</p>
			</div>
			<button
				bind:this={closeButton}
				type="button"
				class="close"
				aria-label="Close font import"
				disabled={importing}
				onclick={onClose}
			>
				<Icon name="close" size={16} />
			</button>
		</header>

		{#if plan.truncated}
			<p class="notice" role="status">
				That selection holds more files than FontNest reviews at once. The first ones are
				listed here; import them and choose the rest afterwards.
			</p>
		{/if}

		<div class="scroller">
			{#if outcomes}
				<ul class="candidates">
					{#each outcomes as outcome (outcome.sourcePath)}
						<li class:blocked={!outcome.installed}>
							<div class="row">
								<span class="file">{outcome.fileName}</span>
								<span class="verdict" class:ready={outcome.installed}>
									{outcome.installed ? 'Installed' : 'Not installed'}
								</span>
							</div>
							<p class="detail">
								{#if outcome.installed}
									{outcome.displayName} is now available to every application on this
									computer.
								{:else if outcome.failure}
									{outcome.failure}
								{:else if outcome.refusal}
									{verdictDetail(outcome.refusal)}
								{/if}
							</p>
						</li>
					{/each}
				</ul>
			{:else}
				<ul class="candidates">
					{#each plan.candidates as candidate (candidate.sourcePath)}
						{@const face = candidate.faces[0]}
						<li class:blocked={verdictTone(candidate.verdict) === 'blocked'}>
							<div class="row">
								<span class="file">{candidate.fileName}</span>
								<span
									class="verdict"
									class:ready={candidate.verdict === 'installable'}
								>
									{verdictLabel(candidate.verdict)}
								</span>
							</div>
							{#if face}
								<p class="face">
									{face.familyName}
									<span class="face-style">{face.subfamilyName}</span>
									{#if face.version}<span class="face-version"
											>{face.version}</span
										>{/if}
								</p>
							{/if}
							<p class="detail">{verdictDetail(candidate.verdict)}</p>
							<p class="facts">
								<span>{formatBytes(candidate.sizeBytes)}</span>
								{#if candidate.faces.length > 1}
									<span>{candidate.faces.length} faces</span>
								{/if}
								{#if embeddingLabel(candidate)}
									<span>{embeddingLabel(candidate)}</span>
								{/if}
								{#if candidate.licence?.url}
									<span class="licence">{candidate.licence.url}</span>
								{/if}
							</p>
							{#if candidate.installedFileName}
								<p class="destination">
									Installs for your account only, as <code
										>{candidate.installedFileName}</code
									>
								</p>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>

		<footer class="panel-foot">
			{#if outcomes}
				<button type="button" class="primary" onclick={onClose}>Done</button>
			{:else}
				<p class="foot-note">
					{#if ready > 0}
						Fonts are copied into your own font folder. Nothing outside it is touched.
					{:else}
						Nothing in this selection can be installed.
					{/if}
				</p>
				<div class="actions">
					<button type="button" onclick={onClose} disabled={importing}>Cancel</button>
					<button
						type="button"
						class="primary"
						disabled={ready === 0 || importing}
						onclick={onConfirm}
					>
						{#if importing}
							Installing…
						{:else}
							Install {ready === 1 ? '1 font' : `${ready} fonts`}
						{/if}
					</button>
				</div>
			{/if}
		</footer>
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
		width: min(680px, 100%);
		max-height: min(760px, 100%);
		flex-direction: column;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-xl);
		background: var(--color-raised);
		box-shadow: var(--shadow-floating);
	}

	.panel-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-lg);
		padding: var(--space-2xl) var(--space-2xl) var(--space-lg);
	}

	.eyebrow {
		margin: 0 0 var(--space-xs);
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	h2 {
		margin: 0;
		font-size: var(--text-title);
		line-height: 1.3;
		letter-spacing: -0.015em;
	}

	.meta {
		margin: var(--space-xs) 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	.close {
		display: grid;
		width: 30px;
		height: 30px;
		flex: none;
		place-items: center;
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		cursor: pointer;
	}

	.close:hover:not(:disabled) {
		border-color: var(--color-border);
		color: var(--color-text);
		background: var(--color-hover);
	}

	.notice {
		margin: 0 var(--space-2xl) var(--space-md);
		padding: var(--space-sm) var(--space-md);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-muted);
		background: var(--color-panel);
		font-size: var(--text-body-sm);
	}

	.scroller {
		overflow-y: auto;
		padding: 0 var(--space-2xl);
	}

	.candidates {
		display: flex;
		flex-direction: column;
		margin: 0;
		padding: 0;
		gap: var(--space-md);
		list-style: none;
	}

	.candidates li {
		padding: var(--space-md);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-surface);
	}

	.candidates li.blocked {
		background: var(--color-panel);
	}

	.row {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-md);
	}

	.file {
		font-size: var(--text-body);
		font-weight: 650;
		overflow-wrap: anywhere;
	}

	.verdict {
		flex: none;
		color: var(--color-muted);
		font-size: var(--text-label);
		font-weight: 650;
	}

	.verdict.ready {
		color: var(--color-success);
	}

	.face {
		margin: var(--space-xs) 0 0;
		font-size: var(--text-body-sm);
	}

	.face-style,
	.face-version {
		margin-left: var(--space-sm);
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.detail {
		margin: var(--space-xs) 0 0;
		max-width: 62ch;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.5;
	}

	.facts {
		display: flex;
		flex-wrap: wrap;
		margin: var(--space-sm) 0 0;
		gap: var(--space-sm) var(--space-md);
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.licence {
		overflow-wrap: anywhere;
	}

	.destination {
		margin: var(--space-sm) 0 0;
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.destination code {
		font-family: ui-monospace, 'Cascadia Mono', 'Segoe UI Mono', monospace;
	}

	.panel-foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-lg);
		padding: var(--space-lg) var(--space-2xl) var(--space-2xl);
	}

	.foot-note {
		margin: 0;
		max-width: 44ch;
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.actions {
		display: flex;
		flex: none;
		gap: var(--space-sm);
	}

	button:not(.close) {
		padding: 7px 14px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	button:not(.close):hover:not(:disabled) {
		background: var(--color-hover);
	}

	button.primary {
		border-color: transparent;
		color: var(--color-accent-ink);
		background: var(--color-accent);
	}

	button.primary:hover:not(:disabled) {
		background: var(--color-accent-hover);
	}

	button:disabled {
		cursor: not-allowed;
		opacity: 0.55;
	}

	@media (max-width: 700px) {
		.overlay {
			padding: var(--space-md);
		}

		.panel-head,
		.panel-foot {
			padding-right: var(--space-lg);
			padding-left: var(--space-lg);
		}

		.scroller {
			padding: 0 var(--space-lg);
		}

		.panel-foot {
			flex-direction: column;
			align-items: stretch;
		}
	}
</style>
