<script lang="ts" module>
	export type ThemePreference = 'system' | 'light' | 'dark';
	export type DensityPreference = 'comfortable' | 'compact';
</script>

<script lang="ts">
	import {
		appUpdater,
		checkForUpdates,
		installAvailableUpdate,
		progressPercent
	} from '$lib/app-updater';

	import Icon from './Icon.svelte';

	let {
		theme,
		density,
		previewText,
		onTheme,
		onDensity,
		onPreviewText
	}: {
		theme: ThemePreference;
		density: DensityPreference;
		previewText: string;
		onTheme: (value: ThemePreference) => void;
		onDensity: (value: DensityPreference) => void;
		onPreviewText: (value: string) => void;
	} = $props();

	let updatePercent = $derived(progressPercent($appUpdater.downloaded, $appUpdater.total));
	let updateBusy = $derived(
		$appUpdater.status === 'checking' ||
			$appUpdater.status === 'downloading' ||
			$appUpdater.status === 'installing'
	);

	function updateStatusText(): string {
		if ($appUpdater.status === 'checking') return 'Checking GitHub Releases…';
		if ($appUpdater.status === 'current') return 'FontNest is up to date.';
		if ($appUpdater.status === 'available' && $appUpdater.update) {
			return `FontNest ${$appUpdater.update.version} is available.`;
		}
		if ($appUpdater.status === 'downloading') {
			return updatePercent === null
				? 'Downloading the verified update…'
				: `Downloading the verified update… ${updatePercent}%`;
		}
		if ($appUpdater.status === 'installing') {
			return 'Installing the update. FontNest will close shortly.';
		}
		if ($appUpdater.status === 'unsupported') {
			return 'Update checks are available in the desktop app.';
		}
		if ($appUpdater.status === 'error') return $appUpdater.error;
		return 'Updates are checked automatically after FontNest starts.';
	}
</script>

<section class="settings-view" aria-labelledby="settings-title">
	<header class="view-heading">
		<div>
			<p class="section-label">System</p>
			<h1 id="settings-title">Settings</h1>
			<p>Keep the catalogue comfortable for long inspection sessions.</p>
		</div>
	</header>

	<div class="settings-content">
		<section class="setting-row" aria-labelledby="appearance-title">
			<div>
				<h2 id="appearance-title">Appearance</h2>
				<p>Use the selected Quiet Ledger mode, or follow your operating system.</p>
			</div>
			<div class="segmented-control" aria-label="Theme preference">
				<button
					type="button"
					class:active={theme === 'system'}
					aria-pressed={theme === 'system'}
					onclick={() => onTheme('system')}
				>
					<Icon name="monitor" size={16} /> System
				</button>
				<button
					type="button"
					class:active={theme === 'light'}
					aria-pressed={theme === 'light'}
					onclick={() => onTheme('light')}
				>
					<Icon name="sun" size={16} /> Light
				</button>
				<button
					type="button"
					class:active={theme === 'dark'}
					aria-pressed={theme === 'dark'}
					onclick={() => onTheme('dark')}
				>
					<Icon name="moon" size={16} /> Dark
				</button>
			</div>
		</section>

		<section class="setting-row" aria-labelledby="density-title">
			<div>
				<h2 id="density-title">Catalogue density</h2>
				<p>Compact mode fits more families on screen without shrinking controls.</p>
			</div>
			<div class="segmented-control two-up" aria-label="Catalogue density">
				<button
					type="button"
					class:active={density === 'comfortable'}
					aria-pressed={density === 'comfortable'}
					onclick={() => onDensity('comfortable')}
				>
					Comfortable
				</button>
				<button
					type="button"
					class:active={density === 'compact'}
					aria-pressed={density === 'compact'}
					onclick={() => onDensity('compact')}
				>
					Compact
				</button>
			</div>
		</section>

		<section class="setting-row" aria-labelledby="sample-title">
			<div>
				<h2 id="sample-title">Default specimen</h2>
				<p>This text is used in the catalogue and inspector for every family.</p>
			</div>
			<label>
				<span class="sr-only">Default specimen text</span>
				<textarea
					rows="3"
					value={previewText}
					oninput={(event) => onPreviewText(event.currentTarget.value)}></textarea>
			</label>
		</section>

		<section class="setting-row" aria-labelledby="safety-title">
			<div>
				<h2 id="safety-title">Font safety</h2>
				<p>
					FontNest installs only verified online font files after you review and confirm
					them.
				</p>
			</div>
			<div class="safety-note">
				<Icon name="check" size={17} />
				<span
					>Installs are per-user and recorded by FontNest. System fonts remain protected.</span
				>
			</div>
		</section>

		<section class="setting-row" aria-labelledby="updates-title">
			<div>
				<h2 id="updates-title">Application updates</h2>
				<p>
					Updates come from the official FontNest GitHub release feed and are verified
					before installation.
				</p>
			</div>
			<div class="update-control" aria-live="polite">
				<div
					class="update-status"
					class:error={$appUpdater.status === 'error'}
					class:available={$appUpdater.status === 'available'}
				>
					<Icon
						name={$appUpdater.status === 'error'
							? 'alert'
							: $appUpdater.status === 'available'
								? 'upload'
								: 'check'}
						size={17}
					/>
					<div>
						<strong>{updateStatusText()}</strong>
						{#if $appUpdater.update?.notes && $appUpdater.status === 'available'}
							<p class="release-notes">{$appUpdater.update.notes}</p>
						{/if}
					</div>
				</div>

				{#if $appUpdater.status === 'downloading'}
					<progress
						aria-label="Update download progress"
						value={updatePercent ?? undefined}
						max="100"
					></progress>
				{/if}

				<div class="update-actions">
					<button
						type="button"
						class="secondary-action"
						disabled={updateBusy}
						onclick={() => void checkForUpdates()}
					>
						{$appUpdater.status === 'checking' ? 'Checking…' : 'Check for updates'}
					</button>
					{#if $appUpdater.update && ['available', 'error'].includes($appUpdater.status)}
						<button
							type="button"
							class="primary-action"
							onclick={() => void installAvailableUpdate()}
						>
							Download and install {$appUpdater.update.version}
						</button>
					{/if}
				</div>
				{#if $appUpdater.update && ['available', 'error'].includes($appUpdater.status)}
					<p class="restart-note">
						FontNest will close while Windows installs the update.
					</p>
				{/if}
			</div>
		</section>

		<section class="shortcuts" aria-labelledby="shortcuts-title">
			<h2 id="shortcuts-title">Keyboard shortcuts</h2>
			<dl>
				<div>
					<dt>Focus search</dt>
					<dd><kbd>/</kbd></dd>
				</div>
				<div>
					<dt>Clear search</dt>
					<dd><kbd>Esc</kbd></dd>
				</div>
				<div>
					<dt>Move through families</dt>
					<dd><kbd>↑</kbd> <kbd>↓</kbd></dd>
				</div>
			</dl>
		</section>
	</div>
</section>

<style>
	.settings-view {
		min-width: 0;
		min-height: 100%;
		background: var(--color-surface);
	}

	.view-heading {
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
		max-width: 62ch;
		margin: 7px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	.settings-content {
		max-width: 920px;
		padding: 0 28px 40px;
	}

	.setting-row {
		display: grid;
		grid-template-columns: minmax(220px, 0.8fr) minmax(280px, 1fr);
		gap: 32px;
		padding: 24px 0;
		border-bottom: 1px solid var(--color-border);
	}

	h2 {
		margin: 0;
		font-size: var(--text-title);
		line-height: 1.3;
		letter-spacing: -0.015em;
	}

	.setting-row p {
		max-width: 48ch;
		margin: 5px 0 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.5;
	}

	.segmented-control {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		align-self: start;
		padding: 3px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-panel);
	}

	.segmented-control.two-up {
		grid-template-columns: repeat(2, 1fr);
	}

	.segmented-control button {
		display: inline-flex;
		min-height: 36px;
		align-items: center;
		justify-content: center;
		gap: 7px;
		padding: 0 10px;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		font-weight: 600;
		cursor: pointer;
	}

	.segmented-control button:hover {
		color: var(--color-text);
	}

	.segmented-control button.active {
		color: var(--color-text);
		background: var(--color-selected);
	}

	textarea {
		width: 100%;
		min-height: 80px;
		padding: 10px 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font: inherit;
		font-size: var(--text-body-sm);
		line-height: 1.5;
		resize: vertical;
	}

	.safety-note {
		display: flex;
		align-items: flex-start;
		gap: 9px;
		padding: 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-success);
		background: var(--color-panel);
		font-size: var(--text-body-sm);
		line-height: 1.45;
	}

	.safety-note span {
		color: var(--color-text);
	}

	.update-control {
		display: grid;
		align-self: start;
		gap: 12px;
	}

	.update-status {
		display: flex;
		align-items: flex-start;
		gap: 9px;
		min-height: 44px;
		padding: 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-success);
		background: var(--color-panel);
	}

	.update-status.available {
		color: var(--color-warning);
	}

	.update-status.error {
		color: var(--color-danger);
	}

	.update-status strong {
		display: block;
		color: var(--color-text);
		font-size: var(--text-body-sm);
		line-height: 1.45;
	}

	.update-status .release-notes {
		margin-top: 5px;
		white-space: pre-line;
	}

	progress {
		width: 100%;
		height: 7px;
		accent-color: var(--color-accent);
	}

	.update-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.update-actions button {
		min-height: 36px;
		padding: 0 12px;
		border-radius: var(--radius-md);
		font-size: var(--text-label);
		font-weight: 600;
		cursor: pointer;
	}

	.update-actions button:disabled {
		cursor: wait;
		opacity: 0.58;
	}

	.secondary-action {
		border: 1px solid var(--color-border);
		color: var(--color-text);
		background: var(--color-control);
	}

	.secondary-action:hover:not(:disabled) {
		background: var(--color-selected);
	}

	.primary-action {
		border: 1px solid transparent;
		color: var(--color-accent-ink);
		background: var(--color-accent);
	}

	.primary-action:hover {
		filter: brightness(1.04);
	}

	.restart-note {
		margin: -3px 0 0;
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.shortcuts {
		padding-top: 24px;
	}

	.shortcuts dl {
		display: grid;
		max-width: 560px;
		gap: 10px;
		margin: 16px 0 0;
	}

	.shortcuts dl div {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 24px;
		font-size: var(--text-body-sm);
	}

	.shortcuts dd {
		margin: 0;
	}

	kbd {
		display: inline-grid;
		min-width: 28px;
		height: 26px;
		place-items: center;
		padding: 0 7px;
		border: 1px solid var(--color-border);
		border-bottom-width: 2px;
		border-radius: var(--radius-xs);
		background: var(--color-control);
		font: inherit;
		font-size: var(--text-micro);
	}

	@media (max-width: 700px) {
		.view-heading,
		.settings-content {
			padding-right: 18px;
			padding-left: 18px;
		}

		.setting-row {
			grid-template-columns: 1fr;
			gap: 14px;
		}
	}
</style>
