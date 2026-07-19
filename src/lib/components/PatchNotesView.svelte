<script lang="ts">
	import { onMount } from 'svelte';

	import {
		loadReleaseNotes,
		markCurrentVersionSeen,
		releaseNotes
	} from '$lib/release-notes/loader';
	import { releaseStatus, type ReleaseEntry } from '$lib/release-notes/changelog';

	import Icon from './Icon.svelte';

	type InlineToken = { text: string; strong: boolean };

	let entries = $derived(
		($releaseNotes.changelog?.entries ?? []).filter(
			(entry) => entry.summary || entry.sections.some((section) => section.items.length)
		)
	);
	let currentVersion = $derived($releaseNotes.currentVersion);
	let loading = $derived($releaseNotes.status === 'loading' && !$releaseNotes.changelog);

	onMount(() => {
		void loadReleaseNotes();
		markCurrentVersionSeen();
	});

	function statusLabel(entry: ReleaseEntry): { text: string; tone: string } | null {
		switch (releaseStatus(entry, currentVersion)) {
			case 'current':
				return { text: 'Installed', tone: 'current' };
			case 'newer':
				return { text: 'Arrives with update', tone: 'newer' };
			case 'unreleased':
				return { text: 'In development', tone: 'unreleased' };
			default:
				return null;
		}
	}

	function formattedDate(date: string | null): string {
		if (!date) return '';
		const parsed = new Date(`${date}T00:00:00`);
		if (Number.isNaN(parsed.getTime())) return date;
		return parsed.toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'long',
			day: 'numeric'
		});
	}

	function inlineTokens(text: string): InlineToken[] {
		return text
			.split(/(\*\*[^*]+\*\*)/g)
			.filter(Boolean)
			.map((part) =>
				part.startsWith('**') && part.endsWith('**')
					? { text: part.slice(2, -2), strong: true }
					: { text: part, strong: false }
			);
	}
</script>

<section class="patch-notes-view" aria-labelledby="patch-notes-title">
	<header class="view-heading">
		<div>
			<p class="section-label">Release notes</p>
			<h1 id="patch-notes-title">What's new</h1>
			<p class="lede">
				Every change that shipped in FontNest, newest first.
				{#if currentVersion}
					You're running <strong>{currentVersion}</strong>.
				{/if}
			</p>
		</div>
		<button
			type="button"
			class="recheck"
			disabled={$releaseNotes.status === 'loading'}
			onclick={() => void loadReleaseNotes(true)}
		>
			<Icon name="refresh" size={15} />
			<span>{$releaseNotes.status === 'loading' ? 'Checking…' : 'Check again'}</span>
		</button>
	</header>

	<div class="feed-source" aria-live="polite">
		{#if $releaseNotes.source === 'remote'}
			<Icon name="check" size={14} /> Up to date with the FontNest release feed.
		{:else}
			<Icon name="library" size={14} /> Showing the notes bundled with this build.
		{/if}
	</div>

	<div class="notes-content">
		{#if loading}
			<div class="notes-skeleton" aria-hidden="true">
				{#each [0, 1, 2] as row (row)}
					<div class="skeleton-entry">
						<span class="skeleton-spine"></span>
						<div class="skeleton-body"><span></span><span></span><span></span></div>
					</div>
				{/each}
			</div>
		{:else if !entries.length}
			<div class="notes-state" role="status">
				<div class="state-icon"><Icon name="alert" size={20} /></div>
				<h2>Release notes are unavailable</h2>
				<p>FontNest could not read its changelog. Try checking again in a moment.</p>
			</div>
		{:else}
			<ol class="release-list">
				{#each entries as entry (entry.label)}
					{@const status = statusLabel(entry)}
					<li class:is-current={status?.tone === 'current'} class="release">
						<div class="release-spine">
							<div class="version-mark" aria-hidden="true"></div>
							<h2 class="version">{entry.label}</h2>
							{#if entry.date}
								<time class="release-date" datetime={entry.date}
									>{formattedDate(entry.date)}</time
								>
							{/if}
							{#if status}
								<span class={`status-pill ${status.tone}`}>
									{#if status.tone === 'current'}<Icon name="check" size={12} />
									{:else if status.tone === 'newer'}<Icon
											name="upload"
											size={12}
										/>{/if}
									{status.text}
								</span>
							{/if}
						</div>

						<div class="release-body">
							{#if entry.summary}
								<p class="release-summary">{entry.summary}</p>
							{/if}

							{#each entry.sections as section, index (index)}
								{#if section.items.length}
									<section class="change-section">
										<h3>{section.title}</h3>
										<ul>
											{#each section.items as item, itemIndex (itemIndex)}
												<li>
													{#each inlineTokens(item) as token, tokenIndex (tokenIndex)}
														{#if token.strong}<strong
																>{token.text}</strong
															>{:else}{token.text}{/if}
													{/each}
												</li>
											{/each}
										</ul>
									</section>
								{/if}
							{/each}
						</div>
					</li>
				{/each}
			</ol>
		{/if}
	</div>
</section>

<style>
	.patch-notes-view {
		min-width: 0;
		min-height: 100%;
		background: var(--color-surface);
	}

	.view-heading {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-xl);
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

	.lede {
		max-width: 64ch;
		margin: 7px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.5;
	}

	.lede strong {
		color: var(--color-text);
		font-variant-numeric: tabular-nums;
	}

	.recheck {
		display: inline-flex;
		height: 34px;
		flex: none;
		align-items: center;
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
			border-color var(--motion-fast);
	}

	.recheck:hover:not(:disabled) {
		background: var(--color-selected);
	}

	.recheck:disabled {
		cursor: wait;
		opacity: 0.58;
	}

	.feed-source {
		display: flex;
		align-items: center;
		gap: 7px;
		padding: 10px 28px;
		border-bottom: 1px solid var(--color-border);
		color: var(--color-subtle);
		background: var(--color-panel);
		font-size: var(--text-micro);
	}

	.notes-content {
		max-width: 920px;
		padding: 8px 28px 48px;
	}

	.release-list {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.release {
		display: grid;
		grid-template-columns: minmax(150px, 190px) minmax(0, 1fr);
		gap: 32px;
		padding: 28px 0;
		border-bottom: 1px solid var(--color-border);
	}

	.release:last-child {
		border-bottom: 0;
	}

	.release-spine {
		position: relative;
		align-self: start;
		padding-left: 18px;
	}

	.version-mark {
		position: absolute;
		top: 6px;
		left: 0;
		width: 9px;
		height: 9px;
		border: 1.5px solid var(--color-subtle);
		border-radius: 50%;
		background: var(--color-surface);
	}

	.is-current .version-mark {
		border-color: var(--color-accent);
		background: var(--color-accent);
	}

	.version {
		margin: 0;
		font-size: var(--text-title);
		font-weight: 650;
		letter-spacing: -0.01em;
		font-variant-numeric: tabular-nums;
	}

	.release-date {
		display: block;
		margin-top: 3px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.status-pill {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		margin-top: 10px;
		padding: 3px 9px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-muted);
		background: var(--color-panel);
		font-size: var(--text-micro);
		font-weight: 650;
	}

	.status-pill.current {
		border-color: color-mix(in srgb, var(--color-accent) 42%, var(--color-border));
		color: var(--color-text);
		background: color-mix(in srgb, var(--color-accent) 12%, var(--color-panel));
	}

	.status-pill.newer {
		border-color: color-mix(in srgb, var(--color-warning) 42%, var(--color-border));
		color: var(--color-warning);
		background: color-mix(in srgb, var(--color-warning) 10%, var(--color-panel));
	}

	.release-body {
		min-width: 0;
	}

	.release-summary {
		margin: 0 0 18px;
		max-width: 70ch;
		color: var(--color-text);
		font-size: var(--text-body);
		line-height: 1.55;
		text-wrap: pretty;
	}

	.change-section + .change-section {
		margin-top: 18px;
	}

	.change-section h3 {
		margin: 0 0 8px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-weight: 650;
		letter-spacing: 0.045em;
		text-transform: uppercase;
	}

	.change-section ul {
		display: grid;
		gap: 8px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.change-section li {
		position: relative;
		max-width: 72ch;
		padding-left: 18px;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.55;
		text-wrap: pretty;
	}

	.change-section li::before {
		position: absolute;
		top: 0.62em;
		left: 2px;
		width: 5px;
		height: 1.5px;
		border-radius: 1px;
		background: var(--color-subtle);
		content: '';
	}

	.change-section li strong {
		color: var(--color-text);
		font-weight: 650;
	}

	/* Loading + empty */
	.notes-skeleton {
		display: grid;
		gap: 28px;
		padding-top: 20px;
	}

	.skeleton-entry {
		display: grid;
		grid-template-columns: minmax(150px, 190px) minmax(0, 1fr);
		gap: 32px;
	}

	.skeleton-spine {
		height: 16px;
		width: 80px;
		border-radius: var(--radius-xs);
		background: var(--color-skeleton);
	}

	.skeleton-body {
		display: grid;
		gap: 10px;
	}

	.skeleton-body span {
		height: 12px;
		border-radius: var(--radius-xs);
		background: var(--color-skeleton);
		animation: notes-pulse 1.25s ease-in-out infinite alternate;
	}

	.skeleton-body span:nth-child(1) {
		width: 92%;
	}

	.skeleton-body span:nth-child(2) {
		width: 78%;
	}

	.skeleton-body span:nth-child(3) {
		width: 60%;
	}

	.notes-state {
		display: grid;
		min-height: 280px;
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
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-muted);
		background: var(--color-panel);
	}

	.notes-state h2 {
		margin: 4px 0 0;
		font-size: var(--text-heading-sm);
	}

	.notes-state p {
		max-width: 46ch;
		margin: 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	@keyframes notes-pulse {
		50% {
			opacity: 0.45;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.skeleton-body span {
			animation: none;
		}
	}

	@media (max-width: 700px) {
		.view-heading,
		.feed-source,
		.notes-content {
			padding-right: 18px;
			padding-left: 18px;
		}

		.release,
		.skeleton-entry {
			grid-template-columns: 1fr;
			gap: 14px;
		}

		.release-spine {
			padding-left: 18px;
		}
	}
</style>
