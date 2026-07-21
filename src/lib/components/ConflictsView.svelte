<script lang="ts">
	import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
	import { familyOrigin, fontOrigin, includesSystemFont } from '$lib/fonts/font-origin';
	import Icon from './Icon.svelte';

	let {
		families,
		onInspect
	}: {
		families: FontFamilySummary[];
		onInspect: (familyId: string) => void;
	} = $props();
</script>

<section class="conflicts-view" aria-labelledby="conflicts-title">
	<header class="view-heading">
		<div>
			<h1 id="conflicts-title">Potential conflicts</h1>
			<p>
				Families are flagged when the same weight and style appear in more than one font
				file. Nothing is changed automatically.
			</p>
		</div>
		<span class="count-chip"
			>{families.length} {families.length === 1 ? 'family' : 'families'}</span
		>
	</header>

	{#if families.length}
		<div class="conflict-list">
			{#each families as family (family.id)}
				<article>
					<div class="conflict-summary">
						<div class="warning-mark"><Icon name="alert" size={18} /></div>
						<div>
							<h2>{family.name}</h2>
							<p>
								{family.faceCount} faces across {family.fileCount} files · {familyOrigin(
									family.origins
								).label}
							</p>
						</div>
						<button type="button" onclick={() => onInspect(family.id)}>
							Inspect family <Icon name="chevron" size={15} />
						</button>
					</div>

					{#if includesSystemFont(family.origins)}
						<p class="system-note">
							Part of this family ships with your operating system. Only the files you
							installed are safe to remove.
						</p>
					{/if}

					<div class="face-table" role="table" aria-label={`${family.name} file details`}>
						<div class="face-head" role="row">
							<span role="columnheader">Face</span>
							<span role="columnheader">File</span>
							<span role="columnheader">Origin</span>
						</div>
						{#each family.faces.slice(0, 8) as face (face.id)}
							<div class="face-row" role="row">
								<span role="cell">{face.styleName}</span>
								<span role="cell" title={face.fileName}>{face.fileName}</span>
								<span role="cell">{fontOrigin(face.origin).label}</span>
							</div>
						{/each}
					</div>
				</article>
			{/each}
		</div>
	{:else}
		<div class="empty-state">
			<div class="empty-icon"><Icon name="check" size={23} /></div>
			<h2>No potential conflicts found</h2>
			<p>
				The current catalogue does not contain repeated family, weight, and style
				combinations across multiple files.
			</p>
		</div>
	{/if}
</section>

<style>
	.system-note {
		margin: -4px 16px 14px 64px;
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.conflicts-view {
		min-width: 0;
		min-height: 100%;
		background: var(--color-surface);
	}

	.view-heading {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 24px;
		padding: 24px 28px 20px;
		border-bottom: 1px solid var(--color-border);
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
		margin: 5px 0 0;
		font-size: var(--text-heading);
		line-height: 1.2;
		letter-spacing: -0.03em;
	}

	.view-heading p:last-child {
		max-width: 66ch;
		margin: 7px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.5;
	}

	.count-chip {
		display: inline-flex;
		min-height: 28px;
		align-items: center;
		padding: 0 10px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-warning);
		background: var(--color-panel);
		font-size: var(--text-label);
		font-weight: 600;
		white-space: nowrap;
	}

	.conflict-list {
		display: grid;
		gap: 16px;
		max-width: 980px;
		padding: 24px 28px 48px;
	}

	article {
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-raised);
		overflow: hidden;
	}

	.conflict-summary {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		padding: 16px;
	}

	.warning-mark {
		display: grid;
		width: 36px;
		height: 36px;
		place-items: center;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-warning);
		background: var(--color-panel);
	}

	h2 {
		margin: 0;
		font-size: var(--text-title);
		line-height: 1.3;
		letter-spacing: -0.015em;
	}

	.conflict-summary p {
		margin: 4px 0 0;
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.conflict-summary button {
		display: inline-flex;
		min-height: 36px;
		align-items: center;
		gap: 6px;
		padding: 0 10px 0 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 600;
		cursor: pointer;
	}

	.conflict-summary button:hover {
		background: var(--color-selected);
	}

	.face-table {
		border-top: 1px solid var(--color-border);
	}

	.face-head,
	.face-row {
		display: grid;
		grid-template-columns: minmax(120px, 0.7fr) minmax(180px, 1.4fr) minmax(110px, 0.6fr);
		align-items: center;
		gap: 16px;
		padding: 0 16px;
	}

	.face-head {
		min-height: 30px;
		color: var(--color-subtle);
		background: var(--color-panel);
		font-size: var(--text-micro);
		font-weight: 650;
	}

	.face-row {
		min-height: 38px;
		border-top: 1px solid var(--color-border);
		font-size: var(--text-body-sm);
	}

	.face-row span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.face-row span:nth-child(2) {
		color: var(--color-muted);
	}

	.face-row span:last-child {
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.empty-state {
		display: grid;
		max-width: 480px;
		place-items: center;
		margin: 80px auto;
		padding: 32px;
		text-align: center;
	}

	.empty-icon {
		display: grid;
		width: 52px;
		height: 52px;
		place-items: center;
		margin-bottom: 16px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-success);
		background: var(--color-raised);
	}

	.empty-state p {
		max-width: 54ch;
		margin: 8px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.55;
	}

	@media (max-width: 700px) {
		.view-heading,
		.conflict-list {
			padding-right: 18px;
			padding-left: 18px;
		}

		.conflict-summary {
			grid-template-columns: auto minmax(0, 1fr);
		}

		.conflict-summary button {
			grid-column: 1 / -1;
			justify-content: center;
		}

		.face-head,
		.face-row {
			grid-template-columns: minmax(64px, 0.6fr) minmax(96px, 1.1fr) minmax(76px, 0.6fr);
			gap: 10px;
		}

		.system-note {
			margin-left: 18px;
		}
	}

	.view-heading {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 24px;
		padding: 24px 28px 20px;
		border-bottom: 1px solid var(--color-border);
	}

	h1 {
		margin: 0;
		font-size: var(--text-heading);
		line-height: 1.2;
		letter-spacing: -0.03em;
	}

	.view-heading p:last-child {
		max-width: 66ch;
		margin: 7px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.5;
	}

	.count-chip {
		display: inline-flex;
		min-height: 28px;
		align-items: center;
		padding: 0 10px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-warning);
		background: var(--color-panel);
		font-size: var(--text-label);
		font-weight: 600;
		white-space: nowrap;
	}
</style>
