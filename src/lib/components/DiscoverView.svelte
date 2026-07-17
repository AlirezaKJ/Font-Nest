<script lang="ts">
	import { onDestroy, onMount } from 'svelte';

	import type { GoogleFontFamilyDetails } from '$lib/bindings/GoogleFontFamilyDetails';
	import type { GoogleFontFamilySummary } from '$lib/bindings/GoogleFontFamilySummary';
	import type { GoogleFontPage } from '$lib/bindings/GoogleFontPage';
	import {
		getGoogleFontDetails,
		installGoogleFont,
		listGoogleFonts,
		prepareGoogleFontPreview
	} from '$lib/tauri/commands';

	import Icon from './Icon.svelte';

	const PAGE_SIZE = 60;
	const CATEGORIES = ['All', 'sans-serif', 'serif', 'monospace', 'display', 'handwriting'];
	const SAMPLE_FAMILIES: GoogleFontFamilySummary[] = (
		[
			['gf:inter', 'Inter', 'sans-serif'],
			['gf:jetbrains-mono', 'JetBrains Mono', 'monospace'],
			['gf:noto-sans', 'Noto Sans', 'sans-serif'],
			['gf:playfair-display', 'Playfair Display', 'serif'],
			['gf:roboto', 'Roboto', 'sans-serif'],
			['gf:source-serif-4', 'Source Serif 4', 'serif']
		] satisfies Array<[string, string, string]>
	).map(([id, family, category]) => ({
		id,
		family,
		category,
		subsets: ['latin', 'latin-ext'],
		license: 'OFL-1.1',
		artifactCount: 2,
		previewArtifactId: `${id}:preview`,
		installed: false
	}));

	let {
		installedFamilyNames,
		onInstalled,
		onToast
	}: {
		installedFamilyNames: string[];
		onInstalled: () => void | Promise<void>;
		onToast: (message: string, tone: 'success' | 'error') => void;
	} = $props();

	let nativeMode = $state(false);
	let page = $state<GoogleFontPage | null>(null);
	let details = $state<GoogleFontFamilyDetails | null>(null);
	let selectedFamilyId = $state<string | null>(null);
	let selectedArtifactIds = $state<string[]>([]);
	let search = $state('');
	let category = $state('All');
	let loadingCatalogue = $state(true);
	let loadingDetails = $state(false);
	let loadingPreview = $state(false);
	let installing = $state(false);
	let confirmingInstall = $state(false);
	let catalogueError = $state('');
	let detailsError = $state('');
	let previewFamily = $state('');
	let loadedPreviewFace: FontFace | null = null;
	let searchTimer: ReturnType<typeof setTimeout> | undefined;
	let catalogueRequest = 0;
	let detailsRequest = 0;

	let families = $derived(page?.families ?? []);
	let selectedFamily = $derived(
		families.find((family) => family.id === selectedFamilyId) ?? null
	);
	let localFamilyPresent = $derived(
		selectedFamily
			? installedFamilyNames.some(
					(name) => name.toLocaleLowerCase() === selectedFamily.family.toLocaleLowerCase()
				)
			: false
	);
	let previewStyle = $derived(
		previewFamily ? `font-family: "${previewFamily}";` : 'font-family: system-ui, sans-serif;'
	);
	let selectedBytes = $derived(
		(details?.artifacts ?? [])
			.filter((artifact) => selectedArtifactIds.includes(artifact.id))
			.reduce((total, artifact) => total + artifact.sizeBytes, 0)
	);

	onMount(() => {
		nativeMode = '__TAURI_INTERNALS__' in window;
		void loadCatalogue(true);
	});

	onDestroy(() => {
		if (searchTimer) clearTimeout(searchTimer);
		clearPreview();
	});

	async function loadCatalogue(reset: boolean) {
		const requestId = ++catalogueRequest;
		loadingCatalogue = true;
		catalogueError = '';

		if (!nativeMode) {
			const query = search.trim().toLocaleLowerCase();
			const matching = SAMPLE_FAMILIES.filter(
				(family) =>
					(!query || family.family.toLocaleLowerCase().includes(query)) &&
					(category === 'All' || family.category === category)
			);
			page = {
				families: matching,
				total: matching.length,
				offset: 0,
				limit: PAGE_SIZE,
				snapshot: 'browser fixture'
			};
			loadingCatalogue = false;
			await selectFirstAvailable();
			return;
		}

		try {
			const offset = reset ? 0 : (page?.families.length ?? 0);
			const response = await listGoogleFonts({
				query: search.trim(),
				category,
				offset,
				limit: PAGE_SIZE
			});
			if (requestId !== catalogueRequest) return;
			page = reset
				? response
				: { ...response, families: [...(page?.families ?? []), ...response.families] };
			await selectFirstAvailable();
		} catch (error) {
			if (requestId !== catalogueRequest) return;
			catalogueError = commandErrorMessage(
				error,
				'The Google Fonts catalogue is unavailable.'
			);
		} finally {
			if (requestId === catalogueRequest) loadingCatalogue = false;
		}
	}

	async function selectFirstAvailable() {
		if (!families.some((family) => family.id === selectedFamilyId)) {
			selectedFamilyId = families[0]?.id ?? null;
		}
		if (selectedFamilyId) await loadDetails(selectedFamilyId);
		else details = null;
	}

	async function selectFamily(familyId: string) {
		selectedFamilyId = familyId;
		await loadDetails(familyId);
	}

	async function loadDetails(familyId: string) {
		const requestId = ++detailsRequest;
		loadingDetails = true;
		detailsError = '';
		clearPreview();

		try {
			const response = nativeMode
				? await getGoogleFontDetails(familyId)
				: sampleDetails(
						SAMPLE_FAMILIES.find((family) => family.id === familyId) ??
							SAMPLE_FAMILIES[0]
					);
			if (requestId !== detailsRequest) return;
			details = response;
			selectedArtifactIds = response.artifacts
				.filter((artifact) => !artifact.installed)
				.map((artifact) => artifact.id);
		} catch (error) {
			if (requestId !== detailsRequest) return;
			details = null;
			detailsError = commandErrorMessage(error, 'Font details are unavailable.');
		} finally {
			if (requestId === detailsRequest) loadingDetails = false;
		}
	}

	function updateSearch(value: string) {
		search = value;
		if (searchTimer) clearTimeout(searchTimer);
		searchTimer = setTimeout(() => void loadCatalogue(true), 220);
	}

	function updateCategory(value: string) {
		category = value;
		void loadCatalogue(true);
	}

	function toggleArtifact(artifactId: string) {
		selectedArtifactIds = selectedArtifactIds.includes(artifactId)
			? selectedArtifactIds.filter((id) => id !== artifactId)
			: [...selectedArtifactIds, artifactId];
	}

	async function loadPreview() {
		if (!nativeMode || !details) return;
		loadingPreview = true;
		try {
			const preview = await prepareGoogleFontPreview(details.previewArtifactId);
			const face = new FontFace(preview.fontFamily, `url(${preview.dataUrl})`);
			await face.load();
			clearPreview();
			document.fonts.add(face);
			loadedPreviewFace = face;
			previewFamily = preview.fontFamily;
		} catch (error) {
			onToast(commandErrorMessage(error, 'FontNest could not load that preview.'), 'error');
		} finally {
			loadingPreview = false;
		}
	}

	function clearPreview() {
		if (loadedPreviewFace) document.fonts.delete(loadedPreviewFace);
		loadedPreviewFace = null;
		previewFamily = '';
	}

	async function confirmInstall() {
		if (!details || !selectedArtifactIds.length || !nativeMode) return;
		installing = true;
		try {
			const result = await installGoogleFont(details.id, selectedArtifactIds);
			const installedIds = new Set([
				...result.installedArtifactIds,
				...result.alreadyInstalledArtifactIds
			]);
			details = {
				...details,
				artifacts: details.artifacts.map((artifact) => ({
					...artifact,
					installed: artifact.installed || installedIds.has(artifact.id)
				}))
			};
			if (page) {
				page = {
					...page,
					families: page.families.map((family) =>
						family.id === result.familyId ? { ...family, installed: true } : family
					)
				};
			}
			selectedArtifactIds = [];
			confirmingInstall = false;
			onToast(
				result.installedArtifactIds.length
					? `${result.familyName} was installed for your Windows account.`
					: `${result.familyName} is already managed by FontNest.`,
				'success'
			);
			await onInstalled();
		} catch (error) {
			onToast(commandErrorMessage(error, 'FontNest could not install that font.'), 'error');
		} finally {
			installing = false;
		}
	}

	function sampleDetails(family: GoogleFontFamilySummary): GoogleFontFamilyDetails {
		return {
			id: family.id,
			family: family.family,
			category: family.category,
			subsets: family.subsets,
			license: family.license,
			lastModified: 'Desktop preview required',
			version: 'Browser fixture',
			previewArtifactId: `${family.id}:regular`,
			artifacts: [
				{
					id: `${family.id}:regular`,
					fileName: `${family.family.replaceAll(' ', '')}[wght].ttf`,
					style: 'Variable',
					format: 'TrueType',
					sizeBytes: 480_000,
					installed: false
				},
				{
					id: `${family.id}:italic`,
					fileName: `${family.family.replaceAll(' ', '')}-Italic[wght].ttf`,
					style: 'Variable Italic',
					format: 'TrueType',
					sizeBytes: 510_000,
					installed: false
				}
			]
		};
	}

	function commandErrorMessage(error: unknown, fallback: string): string {
		if (typeof error === 'object' && error && 'message' in error) return String(error.message);
		return fallback;
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}
</script>

<section class="discover-view" aria-labelledby="discover-title">
	<header class="discover-header">
		<div>
			<p class="section-label">Online library</p>
			<h1 id="discover-title">Discover open fonts</h1>
			<p>Preview and install curated Google Fonts without leaving FontNest.</p>
		</div>
		<div class="source-status">
			<span class:native={nativeMode}></span>
			<div>
				<strong>{nativeMode ? 'Secure desktop catalogue' : 'Browser fixture'}</strong>
				<small>{page?.snapshot ?? 'Loading source snapshot'}</small>
			</div>
		</div>
	</header>

	{#if !nativeMode}
		<div class="desktop-notice" role="note">
			<Icon name="alert" size={16} />
			<span>Run <code>pnpm desktop</code> to download previews or install fonts.</span>
		</div>
	{/if}

	<div class="catalogue-tools">
		<label class="search-control">
			<span class="sr-only">Search online fonts</span>
			<Icon name="search" size={16} />
			<input
				type="search"
				placeholder="Search families, scripts, or licences"
				value={search}
				oninput={(event) => updateSearch(event.currentTarget.value)}
			/>
		</label>
		<label class="category-control">
			<span>Category</span>
			<select
				value={category}
				onchange={(event) => updateCategory(event.currentTarget.value)}
			>
				{#each CATEGORIES as option (option)}
					<option value={option}>{option === 'All' ? 'All categories' : option}</option>
				{/each}
			</select>
		</label>
	</div>

	<div class="discover-workspace">
		<section class="online-catalogue" aria-label="Google Fonts families">
			<div class="catalogue-heading">
				<strong>{page?.total?.toLocaleString() ?? 0} families</strong>
				<span>Google Fonts</span>
			</div>

			{#if loadingCatalogue && !families.length}
				<div class="catalogue-message">Reading the bundled catalogue…</div>
			{:else if catalogueError}
				<div class="catalogue-message error">
					<p>{catalogueError}</p>
					<button type="button" onclick={() => void loadCatalogue(true)}>Try again</button
					>
				</div>
			{:else if !families.length}
				<div class="catalogue-message">No families match this search.</div>
			{:else}
				<div class="online-list">
					{#each families as family (family.id)}
						<button
							type="button"
							class:selected={selectedFamilyId === family.id}
							onclick={() => void selectFamily(family.id)}
						>
							<span class="family-name">
								<strong>{family.family}</strong>
								<small>{family.category} · {family.artifactCount} files</small>
							</span>
							<span class:installed={family.installed} class="availability-dot"
							></span>
						</button>
					{/each}
				</div>
				{#if page && page.families.length < page.total}
					<button
						type="button"
						class="load-more"
						disabled={loadingCatalogue}
						onclick={() => void loadCatalogue(false)}
					>
						{loadingCatalogue ? 'Loading…' : 'Load more families'}
					</button>
				{/if}
			{/if}
		</section>

		<section class="online-inspector" aria-live="polite">
			{#if loadingDetails}
				<div class="inspector-message">Loading family details…</div>
			{:else if detailsError}
				<div class="inspector-message error">{detailsError}</div>
			{:else if details && selectedFamily}
				<header class="family-header">
					<div>
						<div class="family-badges">
							<span>{details.category}</span>
							<span>{details.license}</span>
							{#if details.artifacts.every((artifact) => artifact.installed)}
								<span class="installed-badge"
									><Icon name="check" size={12} /> Managed</span
								>
							{:else if localFamilyPresent}
								<span>Found locally</span>
							{/if}
						</div>
						<h2>{details.family}</h2>
						<p>{details.subsets.slice(0, 7).join(' · ')}</p>
					</div>
					<button
						type="button"
						class="preview-action"
						disabled={!nativeMode || loadingPreview}
						onclick={() => void loadPreview()}
					>
						<Icon name="font" size={15} />
						{loadingPreview
							? 'Loading preview…'
							: previewFamily
								? 'Reload preview'
								: 'Load preview'}
					</button>
				</header>

				<div
					class:loaded={Boolean(previewFamily)}
					class="remote-specimen"
					style={previewStyle}
				>
					<span>Ag</span>
					<p>Pack my box with five dozen liquor jugs.</p>
					<small
						>{previewFamily
							? 'Downloaded preview · not installed'
							: 'Load the trusted preview file'}</small
					>
				</div>

				<section class="artifact-section" aria-labelledby="styles-title">
					<div class="artifact-heading">
						<div>
							<h3 id="styles-title">Desktop font files</h3>
							<p>Select the files to install for your Windows account.</p>
						</div>
						<span>{details.version}</span>
					</div>
					<div class="artifact-list">
						{#each details.artifacts as artifact (artifact.id)}
							<label class:installed={artifact.installed}>
								<input
									type="checkbox"
									checked={artifact.installed ||
										selectedArtifactIds.includes(artifact.id)}
									disabled={artifact.installed || !nativeMode}
									onchange={() => toggleArtifact(artifact.id)}
								/>
								<span>
									<strong>{artifact.style}</strong>
									<small>{artifact.fileName}</small>
								</span>
								<small
									>{artifact.installed
										? 'Installed'
										: formatBytes(artifact.sizeBytes)}</small
								>
							</label>
						{/each}
					</div>
				</section>

				<footer class="install-footer">
					<div>
						<strong
							>{selectedArtifactIds.length} selected · {formatBytes(
								selectedBytes
							)}</strong
						>
						<small>Per-user install · no administrator access</small>
					</div>
					<button
						type="button"
						class="install-action"
						disabled={!nativeMode || !selectedArtifactIds.length || installing}
						onclick={() => (confirmingInstall = true)}
					>
						{installing ? 'Installing…' : 'Review installation'}
					</button>
				</footer>
			{:else}
				<div class="inspector-message">
					Choose a family to inspect its files and licence.
				</div>
			{/if}
		</section>
	</div>
</section>

{#if confirmingInstall && details}
	<div
		class="modal-backdrop"
		role="presentation"
		onclick={(event) => {
			if (event.currentTarget === event.target && !installing) confirmingInstall = false;
		}}
		onkeydown={(event) => {
			if (event.key === 'Escape' && !installing) confirmingInstall = false;
		}}
	>
		<div
			class="install-dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="install-dialog-title"
		>
			<div class="dialog-icon"><Icon name="font" size={20} /></div>
			<h2 id="install-dialog-title">Install {details.family}?</h2>
			<p>
				FontNest will download and verify {selectedArtifactIds.length}
				{selectedArtifactIds.length === 1 ? 'file' : 'files'}, then register them for your
				Windows account.
			</p>
			<dl>
				<div>
					<dt>Source</dt>
					<dd>Google Fonts</dd>
				</div>
				<div>
					<dt>Licence</dt>
					<dd>{details.license}</dd>
				</div>
				<div>
					<dt>Scope</dt>
					<dd>Current user</dd>
				</div>
				<div>
					<dt>Download</dt>
					<dd>{formatBytes(selectedBytes)}</dd>
				</div>
			</dl>
			<div class="dialog-actions">
				<button
					type="button"
					disabled={installing}
					onclick={() => (confirmingInstall = false)}
				>
					Cancel
				</button>
				<button
					type="button"
					class="confirm-action"
					disabled={installing}
					onclick={confirmInstall}
				>
					{installing ? 'Installing securely…' : 'Install font files'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.discover-view {
		display: flex;
		width: 100%;
		height: 100%;
		min-height: 0;
		flex-direction: column;
		overflow: hidden;
		background: var(--color-surface);
	}

	.discover-header {
		display: flex;
		flex: none;
		align-items: flex-end;
		justify-content: space-between;
		gap: 24px;
		padding: 20px 24px 16px;
		border-bottom: 1px solid var(--color-border);
	}

	.section-label {
		margin: 0 0 5px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		font-weight: 700;
		letter-spacing: 0.055em;
		text-transform: uppercase;
	}

	h1,
	h2,
	h3,
	p {
		margin-top: 0;
	}

	h1 {
		margin-bottom: 6px;
		font-size: var(--text-heading);
		letter-spacing: -0.035em;
	}

	.discover-header p:last-child,
	.family-header p,
	.artifact-heading p {
		margin-bottom: 0;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	.source-status {
		display: flex;
		min-width: 205px;
		align-items: center;
		gap: 9px;
		padding: 9px 11px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-panel);
	}

	.source-status > span {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--color-warning);
	}

	.source-status > span.native {
		background: var(--color-success);
	}

	.source-status div {
		display: grid;
		min-width: 0;
		gap: 2px;
	}

	.source-status strong,
	.source-status small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.source-status strong {
		font-size: var(--text-label);
	}

	.source-status small {
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.desktop-notice {
		display: flex;
		flex: none;
		align-items: center;
		gap: 9px;
		padding: 10px 24px;
		border-bottom: 1px solid color-mix(in srgb, var(--color-warning) 38%, var(--color-border));
		color: var(--color-warning);
		background: color-mix(in srgb, var(--color-warning) 8%, transparent);
		font-size: var(--text-body-sm);
	}

	.desktop-notice code {
		font-size: var(--text-label);
	}

	.catalogue-tools {
		display: grid;
		flex: none;
		grid-template-columns: minmax(260px, 1fr) 190px;
		gap: 12px;
		padding: 12px 24px;
		border-bottom: 1px solid var(--color-border);
	}

	.search-control {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		align-items: center;
		gap: 9px;
		padding: 0 11px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-muted);
		background: var(--color-control);
	}

	.search-control input {
		height: 40px;
		border: 0;
		outline: 0;
		color: var(--color-text);
		background: transparent;
		font-size: var(--text-body-sm);
	}

	.category-control {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		align-items: center;
		gap: 8px;
		padding-left: 10px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-muted);
		background: var(--color-control);
		font-size: var(--text-micro);
		font-weight: 650;
	}

	.category-control select {
		height: 40px;
		border: 0;
		outline: 0;
		color: var(--color-text);
		background: transparent;
		font-size: var(--text-body-sm);
	}

	.discover-workspace {
		display: grid;
		min-height: 0;
		flex: 1;
		grid-template-columns: minmax(250px, 0.72fr) minmax(440px, 1.28fr);
		overflow: hidden;
		background: var(--color-panel);
	}

	.online-catalogue {
		display: flex;
		min-width: 0;
		min-height: 0;
		flex-direction: column;
		border-right: 1px solid var(--color-border);
	}

	.catalogue-heading {
		display: flex;
		height: 44px;
		align-items: center;
		justify-content: space-between;
		padding: 0 14px;
		border-bottom: 1px solid var(--color-border);
		font-size: var(--text-label);
	}

	.catalogue-heading span {
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.online-list {
		min-height: 0;
		flex: 1;
		overflow-y: auto;
	}

	.online-list button {
		display: grid;
		width: 100%;
		min-height: 58px;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 10px;
		padding: 8px 14px;
		border: 0;
		border-bottom: 1px solid var(--color-border);
		color: var(--color-text);
		background: transparent;
		text-align: left;
		cursor: pointer;
		transition: background var(--motion-fast);
	}

	.online-list button:hover {
		background: var(--color-hover);
	}

	.online-list button.selected {
		background: var(--color-selected);
	}

	.family-name {
		display: grid;
		min-width: 0;
		gap: 3px;
	}

	.family-name strong,
	.family-name small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.family-name strong {
		font-size: var(--text-body-sm);
	}

	.family-name small {
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.availability-dot {
		width: 7px;
		height: 7px;
		border: 1px solid var(--color-subtle);
		border-radius: 50%;
	}

	.availability-dot.installed {
		border-color: var(--color-success);
		background: var(--color-success);
	}

	.load-more,
	.catalogue-message button {
		min-height: 36px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.load-more {
		width: calc(100% - 24px);
		margin: 12px;
	}

	.catalogue-message,
	.inspector-message {
		display: grid;
		min-height: 180px;
		place-items: center;
		padding: 22px;
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		text-align: center;
	}

	.catalogue-message.error,
	.inspector-message.error {
		color: var(--color-danger);
	}

	.online-inspector {
		min-width: 0;
		min-height: 0;
		overflow-y: auto;
		padding: clamp(20px, 2.5vw, 32px);
		background: var(--color-surface);
	}

	.family-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 20px;
	}

	.family-badges {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-bottom: 10px;
	}

	.family-badges span {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		padding: 3px 7px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.family-badges .installed-badge {
		color: var(--color-success);
	}

	.family-header h2 {
		margin-bottom: 5px;
		font-size: 1.75rem;
		letter-spacing: -0.04em;
	}

	.preview-action,
	.install-action,
	.dialog-actions button {
		display: inline-flex;
		min-height: 38px;
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
	}

	.preview-action:disabled,
	.install-action:disabled,
	.dialog-actions button:disabled {
		opacity: 0.5;
	}

	.remote-specimen {
		display: grid;
		min-height: 190px;
		align-content: center;
		gap: 8px;
		margin-top: 24px;
		padding: 24px 0;
		border-top: 1px solid var(--color-border);
		border-bottom: 1px solid var(--color-border);
	}

	.remote-specimen > span {
		font-size: clamp(3.2rem, 8vw, 6rem);
		line-height: 0.9;
		letter-spacing: -0.06em;
	}

	.remote-specimen p {
		margin-bottom: 0;
		font-size: clamp(1.35rem, 2.6vw, 2.15rem);
		line-height: 1.1;
		letter-spacing: -0.035em;
	}

	.remote-specimen small {
		color: var(--color-subtle);
		font-family: inherit;
		font-size: var(--text-micro);
		letter-spacing: normal;
	}

	.artifact-section {
		padding-top: 22px;
	}

	.artifact-heading {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 18px;
	}

	.artifact-heading h3 {
		margin-bottom: 4px;
		font-size: var(--text-title);
	}

	.artifact-heading > span {
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.artifact-list {
		display: grid;
		gap: 0;
		margin-top: 14px;
		border-top: 1px solid var(--color-border);
	}

	.artifact-list label {
		display: grid;
		min-height: 54px;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 11px;
		border-bottom: 1px solid var(--color-border);
		font-size: var(--text-body-sm);
		cursor: pointer;
	}

	.artifact-list label.installed {
		color: var(--color-muted);
		cursor: default;
	}

	.artifact-list input {
		width: 16px;
		height: 16px;
		accent-color: var(--color-accent);
	}

	.artifact-list label > span {
		display: grid;
		min-width: 0;
		gap: 2px;
	}

	.artifact-list small {
		overflow: hidden;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.install-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 20px;
		margin-top: 22px;
		padding-top: 18px;
		border-top: 1px solid var(--color-border);
	}

	.install-footer > div {
		display: grid;
		gap: 3px;
		font-size: var(--text-label);
	}

	.install-footer small {
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.install-action,
	.dialog-actions .confirm-action {
		border-color: var(--color-accent);
		color: var(--color-accent-ink);
		background: var(--color-accent);
	}

	.modal-backdrop {
		position: fixed;
		inset: 0;
		z-index: var(--z-modal-backdrop);
		display: grid;
		place-items: center;
		padding: 20px;
		background: color-mix(in srgb, var(--color-bg) 68%, transparent);
		backdrop-filter: blur(5px);
	}

	.install-dialog {
		width: min(460px, 100%);
		padding: 24px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		background: var(--color-raised);
		box-shadow: var(--shadow-floating);
	}

	.dialog-icon {
		display: grid;
		width: 40px;
		height: 40px;
		place-items: center;
		margin-bottom: 16px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-panel);
	}

	.install-dialog h2 {
		margin-bottom: 7px;
		font-size: var(--text-heading-sm);
	}

	.install-dialog > p {
		color: var(--color-muted);
		font-size: var(--text-body-sm);
		line-height: 1.5;
	}

	.install-dialog dl {
		display: grid;
		gap: 8px;
		margin: 18px 0 0;
		padding: 14px 0;
		border-top: 1px solid var(--color-border);
		border-bottom: 1px solid var(--color-border);
	}

	.install-dialog dl div {
		display: flex;
		justify-content: space-between;
		gap: 20px;
		font-size: var(--text-body-sm);
	}

	.install-dialog dt {
		color: var(--color-muted);
	}

	.install-dialog dd {
		margin: 0;
		font-weight: 650;
	}

	.dialog-actions {
		display: flex;
		justify-content: flex-end;
		gap: 9px;
		margin-top: 20px;
	}

	@media (max-width: 960px) {
		.discover-workspace {
			grid-template-columns: minmax(220px, 0.62fr) minmax(380px, 1.38fr);
		}
	}

	@media (max-width: 760px) {
		.discover-view {
			height: auto;
			min-height: 100%;
			overflow: visible;
		}

		.discover-header,
		.family-header,
		.install-footer {
			align-items: stretch;
			flex-direction: column;
		}

		.source-status {
			width: 100%;
		}

		.catalogue-tools {
			grid-template-columns: 1fr;
		}

		.discover-workspace {
			flex: none;
			grid-template-columns: 1fr;
		}

		.online-catalogue {
			border-right: 0;
			border-bottom: 1px solid var(--color-border);
		}

		.online-list {
			flex: none;
			max-height: 280px;
		}

		.online-inspector {
			overflow: visible;
		}

		.preview-action,
		.install-action {
			width: 100%;
		}
	}
</style>
