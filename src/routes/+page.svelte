<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { onMount, tick } from 'svelte';

	import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
	import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
	import type { ImportOutcome } from '$lib/bindings/ImportOutcome';
	import type { ImportPlan } from '$lib/bindings/ImportPlan';
	import type { ValidatedLocalFont } from '$lib/bindings/ValidatedLocalFont';
	import { checkForUpdates } from '$lib/app-updater';
	import { getConflictDestination } from '$lib/conflict-navigation';
	import AppNavigation, { type AppView } from '$lib/components/AppNavigation.svelte';
	import AppTitleBar from '$lib/components/AppTitleBar.svelte';
	import ConflictsView from '$lib/components/ConflictsView.svelte';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import FontImportReview from '$lib/components/FontImportReview.svelte';
	import LocalFontPreview from '$lib/components/LocalFontPreview.svelte';
	import DiscoverView from '$lib/components/DiscoverView.svelte';
	import FontPreviewView from '$lib/components/FontPreviewView.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import type { DiscoverFilterOption } from '$lib/components/DiscoverFilterMenu.svelte';
	import type { FilterGroup } from '$lib/components/FilterPopover.svelte';
	import LibraryView from '$lib/components/LibraryView.svelte';
	import PatchNotesView from '$lib/components/PatchNotesView.svelte';
	import SettingsView, {
		type DensityPreference,
		type ThemePreference
	} from '$lib/components/SettingsView.svelte';
	import { createBrowserCatalogue } from '$lib/catalogue/browser-catalogue';
	import { writeClipboardText } from '$lib/context-menu/clipboard';
	import { faceContextMenu, familyContextMenu } from '$lib/context-menu/entries';
	import { FONT_ORIGIN_ORDER, fontOrigin } from '$lib/fonts/font-origin';
	import { importReviewed, reviewChosenFonts } from '$lib/fonts/import';
	import { nearestWeight } from '$lib/fonts/weights';
	import {
		type ActiveLibraryFilter,
		type LibraryFilterKey,
		type LibrarySortOrder,
		filterFamilies,
		formatValues
	} from '$lib/library/filters';
	import { EMPTY_INVENTORY, formatBytes, refusalDetail } from '$lib/fonts/managed';
	import { importLocalFontPreview, releaseLocalFontPreview } from '$lib/fonts/local-fonts';
	import { hasUnseenRelease } from '$lib/release-notes/loader';
	import { reorderIds, type ReorderPosition } from '$lib/reorder';
	import {
		parseSession,
		reconcileSession,
		restorableSortOrder,
		restorableView,
		type Session
	} from '$lib/session';
	import * as preferencesStore from '$lib/preferences/store';
	import {
		discardManagedFont,
		managedFontInventory,
		removeManagedFont,
		restoreManagedFont
	} from '$lib/tauri/commands';
	import { fontFaceFilePath, revealFontFaceFile, scanInstalledFonts } from '$lib/tauri/commands';

	const PAGE_SIZE = 120;
	const DEFAULT_PREVIEW = 'What is life but a fevered dream';
	const DEFAULT_SPECIMEN_SIZE = 96;
	const DEFAULT_SPECIMEN_WEIGHT = 400;
	const UPDATE_CHECK_DELAY_MS = 8_000;
	const PREFERENCES_SAVE_DELAY_MS = 400;
	// Saved family IDs from before opaque IDs shipped can never match a family again, so they are
	// dropped on load rather than carried forever in the preference blob.
	const FAMILY_ID_PATTERN = /^family:[0-9a-f]{32}$/;

	const SPACING_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'Any spacing' },
		{ value: 'proportional', label: 'Proportional' },
		{ value: 'monospaced', label: 'Monospaced' }
	];
	const TECHNOLOGY_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'Any technology' },
		{
			value: 'variable',
			label: 'Variable',
			description: 'One file covers a range of weights or widths'
		},
		{ value: 'static', label: 'Static', description: 'One file per style' }
	];
	const STATUS_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'all', label: 'Any status' },
		{ value: 'conflict', label: 'Conflicts only', description: 'Families with duplicate files' }
	];
	const SORT_OPTIONS: DiscoverFilterOption[] = [
		{ value: 'name-asc', label: 'Name A–Z' },
		{ value: 'name-desc', label: 'Name Z–A' },
		{ value: 'styles', label: 'Most styles' },
		{ value: 'faces', label: 'Most faces' }
	];

	type CatalogueMode = 'native' | 'browser';
	type SpecimenMode = 'names' | 'custom';
	type Toast = { message: string; tone: 'success' | 'error' };

	let view = $state<AppView>('library');
	let unseenRelease = $state(false);

	function navigate(nextView: AppView) {
		if (nextView === 'whatsNew') unseenRelease = false;
		view = nextView;
	}

	// Settings is the only place that shows the inventory, so it is read when Settings opens
	// rather than kept up to date in the background.
	$effect(() => {
		if (view === 'settings') void refreshManagedInventory();
	});

	// Everything the session holds, written back through the same debounced path the specimen
	// text uses. One effect beats a save call at every filter, toggle and scroll handler.
	$effect(() => {
		void [
			view,
			selectedFamilyId,
			search,
			originFilter,
			formatFilter,
			technologyFilter,
			spacingFilter,
			statusFilter,
			sortOrder,
			specimenMode,
			specimenSize,
			specimenWeight,
			previewSize,
			previewWeight,
			displayLimit
		];
		// Until the stored session has been applied, the values above are still defaults and
		// writing them would throw away what the last session left.
		if (pendingSession) return;
		queuePreferencesSave();
	});

	// The stored session is applied once, and only after a catalogue exists to check it against:
	// a family can be uninstalled between one launch and the next, and reopening the preview of a
	// font that is no longer installed would be a blank panel with nothing to explain it.
	$effect(() => {
		// Read what this depends on before anything can return early. A guard that short-circuits
		// ahead of a reactive read registers no dependency on it, and the effect then never runs
		// again: the first version of this restored nothing at all for that reason.
		const scanning = loading;
		const scanned = catalogue;
		if (!pendingSession || scanning) return;
		if (!scanned) {
			// The scan failed. There is nothing to check a saved family against, so let the
			// session go rather than holding every later write hostage to it.
			pendingSession = null;
			return;
		}
		const families = scanned.families;
		const session = reconcileSession(pendingSession, (familyId) =>
			families.some((family) => family.id === familyId)
		);
		pendingSession = null;

		selectedFamilyId = session.selectedFamilyId ?? null;
		if (session.displayLimit !== undefined) {
			displayLimit = Math.max(displayLimit, session.displayLimit);
		}
		if (session.view) view = session.view;

		const scrollTop = session.scrollTop ?? 0;
		if (session.view !== 'library' || scrollTop <= 0) return;
		// Wait for the rows to exist before scrolling past them, or the container clamps the
		// position to whatever short list has rendered so far.
		void tick().then(() => libraryScrollElement?.scrollTo({ top: scrollTop }));
	});

	$effect(() => {
		if (typeof window === 'undefined') return;
		const query = window.matchMedia('(prefers-reduced-motion: reduce)');
		const syncPreference = () => (prefersReducedMotion = query.matches);
		syncPreference();
		query.addEventListener('change', syncPreference);
		return () => query.removeEventListener('change', syncPreference);
	});

	let catalogue = $state<FontCatalogue | null>(null);
	let catalogueMode = $state<CatalogueMode>('browser');
	let loading = $state(true);
	let errorMessage = $state('');
	let selectedFamilyId = $state<string | null>(null);
	let search = $state('');
	let originFilter = $state('all');
	let formatFilter = $state('all');
	let technologyFilter = $state('all');
	let spacingFilter = $state('all');
	let statusFilter = $state('all');
	let sortOrder = $state('name-asc');
	let specimenMode = $state<SpecimenMode>('names');
	let specimenSize = $state(DEFAULT_SPECIMEN_SIZE);
	let specimenWeight = $state(DEFAULT_SPECIMEN_WEIGHT);
	let displayLimit = $state(PAGE_SIZE);
	let previewText = $state(DEFAULT_PREVIEW);
	let previewSize = $state(64);
	let previewWeight = $state(400);
	let theme = $state<ThemePreference>('dark');
	let resolvedTheme = $state<'light' | 'dark'>('dark');
	let density = $state<DensityPreference>('comfortable');
	let focusOutlines = $state(false);
	let sidebarCollapsed = $state(false);
	let pinnedFamilyIds = $state<string[]>([]);
	let toast = $state<Toast | null>(null);
	let localPreview = $state<ValidatedLocalFont | null>(null);
	// The review a person is looking at, what the import then did, and whether it is still running.
	let importPlan = $state<ImportPlan | null>(null);
	let importOutcomes = $state<ImportOutcome[] | null>(null);
	let importRunning = $state(false);
	// What FontNest is looking after, and which row is mid-operation so only its own button says so.
	let managedInventory = $state(EMPTY_INVENTORY);
	let managedBusyId = $state<string | null>(null);
	let prefersReducedMotion = $state(false);
	let toastTimer: ReturnType<typeof setTimeout> | undefined;
	let updateCheckTimer: ReturnType<typeof setTimeout> | undefined;
	let preferencesTimer: ReturnType<typeof setTimeout> | null = null;
	// One toast is enough when the store itself is failing.
	let preferencesWriteFailed = false;
	// Where the last session left off, held until a catalogue exists to check it against, then
	// applied once. A family can be uninstalled between launches, so nothing is taken on trust.
	let pendingSession = $state<Partial<Session> | null>(null);
	let libraryScrollElement = $state<HTMLElement>();

	let originOptions = $derived.by<DiscoverFilterOption[]>(() => {
		const present = new Set(catalogue?.families.flatMap((family) => family.origins) ?? []);
		return [
			{ value: 'all', label: 'Anywhere' },
			...FONT_ORIGIN_ORDER.filter((origin) => present.has(origin)).map((origin) => ({
				value: origin,
				label: fontOrigin(origin).label,
				description: fontOrigin(origin).description
			}))
		];
	});

	let formatOptions = $derived.by<DiscoverFilterOption[]>(() => [
		{ value: 'all', label: 'All formats' },
		...formatValues(catalogue?.families ?? []).map((value) => ({ value, label: value }))
	]);

	let filterGroups = $derived.by<FilterGroup[]>(() => [
		{ key: 'origin', label: 'Origin', value: originFilter, options: originOptions },
		{ key: 'format', label: 'Format', value: formatFilter, options: formatOptions },
		{
			key: 'technology',
			label: 'Technology',
			value: technologyFilter,
			options: TECHNOLOGY_OPTIONS
		},
		{ key: 'spacing', label: 'Spacing', value: spacingFilter, options: SPACING_OPTIONS },
		{ key: 'status', label: 'Status', value: statusFilter, options: STATUS_OPTIONS }
	]);

	// The trigger no longer shows each filter's value, so these chips are the only place
	// active choices are named. Sort stays out of them: it always has a visible value.
	let activeFilters = $derived.by<ActiveLibraryFilter[]>(() =>
		filterGroups
			.filter((group) => group.value !== 'all')
			.map((group) => ({
				key: group.key as LibraryFilterKey,
				label: optionLabel(group.options, group.value)
			}))
	);

	let hasResettableState = $derived(
		Boolean(search) ||
			activeFilters.length > 0 ||
			sortOrder !== 'name-asc' ||
			specimenMode !== 'names' ||
			specimenSize !== DEFAULT_SPECIMEN_SIZE ||
			specimenWeight !== DEFAULT_SPECIMEN_WEIGHT
	);

	let filteredFamilies = $derived(
		filterFamilies(
			catalogue?.families ?? [],
			search,
			{
				origin: originFilter,
				format: formatFilter,
				technology: technologyFilter,
				spacing: spacingFilter,
				status: statusFilter
			},
			sortOrder as LibrarySortOrder
		)
	);

	let selectedFamily = $derived.by(() => {
		const selected =
			catalogue?.families.find((family) => family.id === selectedFamilyId) ?? null;
		if (view === 'preview') return selected;
		if (
			filteredFamilies.length &&
			!filteredFamilies.some((family) => family.id === selected?.id)
		) {
			return filteredFamilies[0];
		}
		return selected ?? filteredFamilies[0] ?? null;
	});
	let conflictFamilies = $derived(
		catalogue?.families.filter((family) => family.hasConflict) ?? []
	);
	let pinnedFamilies = $derived.by(() =>
		pinnedFamilyIds
			.map((familyId) => catalogue?.families.find((family) => family.id === familyId))
			.filter((family): family is FontFamilySummary => Boolean(family))
	);

	onMount(() => {
		applyTheme();
		applyFocusOutlines();
		// The window is hidden until this resolves, so reading the settings over IPC costs a
		// few milliseconds of hidden window rather than a flash of the wrong theme.
		void loadStoredPreferences().finally(revealWindow);

		const colorScheme = window.matchMedia('(prefers-color-scheme: dark)');
		const handleColorScheme = () => {
			if (theme === 'system') applyTheme();
		};
		const handleKeydown = (event: KeyboardEvent) => {
			const target = event.target as HTMLElement | null;
			const isEditing =
				target?.matches('input, textarea, select, [contenteditable="true"]') ?? false;

			if (event.key === '/' && !isEditing) {
				event.preventDefault();
				view = 'library';
				focusSearch();
			} else if (event.key === 'Escape' && search) {
				search = '';
				displayLimit = PAGE_SIZE;
				focusSearch();
			}
		};

		colorScheme.addEventListener('change', handleColorScheme);
		window.addEventListener('keydown', handleKeydown);
		// Closing the window or hiding it mid-sentence must not lose the specimen text
		// the debounce is still holding.
		window.addEventListener('pagehide', flushPreferences);
		document.addEventListener('visibilitychange', handleVisibilityChange);
		void refreshCatalogue();

		unseenRelease = hasUnseenRelease();
		if (unseenRelease) {
			showToast('FontNest was updated. Open What’s new to see what changed.', 'success');
		}

		if ('__TAURI_INTERNALS__' in window) {
			updateCheckTimer = setTimeout(() => {
				void checkForUpdates().then((update) => {
					if (update) {
						showToast(
							`FontNest ${update.version} is available. Open Settings to install it.`,
							'success'
						);
					}
				});
			}, UPDATE_CHECK_DELAY_MS);
		}

		return () => {
			colorScheme.removeEventListener('change', handleColorScheme);
			window.removeEventListener('keydown', handleKeydown);
			window.removeEventListener('pagehide', flushPreferences);
			document.removeEventListener('visibilitychange', handleVisibilityChange);
			flushPreferences();
			if (toastTimer) clearTimeout(toastTimer);
			if (updateCheckTimer) clearTimeout(updateCheckTimer);
		};
	});

	/**
	 * Reads the settings FontNest owns, applies what is still valid, and says nothing when there
	 * is nothing stored. Values are checked here as well as in the store: the desktop side bounds
	 * what a document can hold, and this decides what this interface is willing to show.
	 */
	async function loadStoredPreferences() {
		const loaded = await preferencesStore.load();
		const saved = loaded.preferences;

		theme = saved.theme;
		density = saved.density;
		focusOutlines = saved.focusOutlines;
		if (saved.previewText.trim()) previewText = saved.previewText;
		sidebarCollapsed = saved.sidebarCollapsed;
		pinnedFamilyIds = [
			...new Set(saved.pinnedFamilyIds.filter((value) => FAMILY_ID_PATTERN.test(value)))
		];

		pendingSession = parseSession(saved.session);
		if (pendingSession.search !== undefined) search = pendingSession.search;
		if (pendingSession.filters) {
			originFilter = pendingSession.filters.origin;
			formatFilter = pendingSession.filters.format;
			technologyFilter = pendingSession.filters.technology;
			spacingFilter = pendingSession.filters.spacing;
			statusFilter = pendingSession.filters.status;
		}
		if (pendingSession.sortOrder) sortOrder = pendingSession.sortOrder;
		if (pendingSession.specimenMode) specimenMode = pendingSession.specimenMode;
		if (pendingSession.specimenSize !== undefined) specimenSize = pendingSession.specimenSize;
		if (pendingSession.specimenWeight !== undefined) {
			specimenWeight = pendingSession.specimenWeight;
		}
		if (pendingSession.previewSize !== undefined) previewSize = pendingSession.previewSize;
		if (pendingSession.previewWeight !== undefined) {
			previewWeight = pendingSession.previewWeight;
		}

		applyTheme();
		applyFocusOutlines();

		// Settings that could not be read are not a silent event: the defaults on screen are not
		// what the person left, and they deserve to know why.
		if (loaded.recovery !== null) {
			showToast('FontNest could not read your saved settings, so it started fresh.', 'error');
		}
	}

	function savePreferences() {
		if (preferencesTimer) {
			clearTimeout(preferencesTimer);
			preferencesTimer = null;
		}
		void preferencesStore
			.save({
				theme,
				density,
				focusOutlines,
				previewText,
				sidebarCollapsed,
				pinnedFamilyIds,
				session: {
					view: restorableView(view),
					selectedFamilyId,
					search,
					filters: {
						origin: originFilter,
						format: formatFilter,
						technology: technologyFilter,
						spacing: spacingFilter,
						status: statusFilter
					},
					sortOrder: restorableSortOrder(sortOrder),
					specimenMode,
					specimenSize,
					specimenWeight,
					previewSize,
					previewWeight,
					displayLimit,
					scrollTop: libraryScrollElement?.scrollTop ?? 0
				} satisfies Session
			})
			.catch((error: unknown) => {
				// One complaint is enough: a failing store would otherwise interrupt on every
				// keystroke that touches a setting.
				if (!preferencesWriteFailed) {
					preferencesWriteFailed = true;
					showToast(commandErrorMessage(error), 'error');
				}
			});
	}

	// Specimen text changes on every keystroke. Serializing the whole preference blob
	// and writing it that often is wasted work, so typing coalesces into one write.
	// Discrete actions (theme, density, saved previews) still persist immediately.
	function queuePreferencesSave() {
		if (preferencesTimer) clearTimeout(preferencesTimer);
		preferencesTimer = setTimeout(savePreferences, PREFERENCES_SAVE_DELAY_MS);
	}

	function flushPreferences() {
		if (preferencesTimer) savePreferences();
	}

	function handleVisibilityChange() {
		if (document.visibilityState === 'hidden') flushPreferences();
	}

	// Focus outlines are opt-in. The attribute flips the --focus-ring tokens so every
	// focus ring in the app shows or hides together.
	function applyFocusOutlines() {
		document.documentElement.dataset.focusOutlines = focusOutlines ? 'on' : 'off';
	}

	function setFocusOutlines(value: boolean) {
		focusOutlines = value;
		applyFocusOutlines();
		savePreferences();
	}

	function applyTheme() {
		const resolved =
			theme === 'system'
				? window.matchMedia('(prefers-color-scheme: dark)').matches
					? 'dark'
					: 'light'
				: theme;
		document.documentElement.dataset.theme = resolved;
		document.documentElement.style.colorScheme = resolved;
		resolvedTheme = resolved;
	}

	function revealWindow() {
		if (!('__TAURI_INTERNALS__' in window)) return;
		// The native window launches hidden (visible:false in tauri.conf) so the user
		// never sees the WebView's blank white background while the bundle loads. Reveal
		// it only once the themed first frame has painted; the double requestAnimationFrame
		// waits one full paint past mount.
		requestAnimationFrame(() => {
			requestAnimationFrame(() => {
				void getCurrentWindow()
					.show()
					.catch((error) =>
						console.error('FontNest could not reveal its window.', error)
					);
			});
		});
	}

	function setTheme(value: ThemePreference) {
		theme = value;
		applyTheme();
		savePreferences();
	}

	function toggleTheme() {
		const resolved = document.documentElement.dataset.theme;
		setTheme(resolved === 'dark' ? 'light' : 'dark');
	}

	function setDensity(value: DensityPreference) {
		density = value;
		savePreferences();
	}

	function setPreviewText(value: string) {
		previewText = value;
		queuePreferencesSave();
	}

	function toggleSidebar() {
		sidebarCollapsed = !sidebarCollapsed;
		savePreferences();
	}

	async function refreshCatalogue() {
		loading = true;
		errorMessage = '';
		const isNative = '__TAURI_INTERNALS__' in window;
		catalogueMode = isNative ? 'native' : 'browser';

		if (!isNative) {
			catalogue = createBrowserCatalogue();
			selectedFamilyId = null;
			previewWeight = nearestWeight(catalogue.families[0]?.weights ?? [400], 400);
			loading = false;
			return;
		}

		try {
			catalogue = await scanInstalledFonts();
			selectedFamilyId = null;
			previewWeight = nearestWeight(catalogue.families[0]?.weights ?? [400], 400);
		} catch (error) {
			catalogue = null;
			errorMessage = commandErrorMessage(error);
		} finally {
			loading = false;
		}
	}

	function commandErrorMessage(error: unknown): string {
		if (typeof error === 'object' && error && 'message' in error) {
			return String(error.message);
		}
		return 'FontNest could not read the installed font catalogue. Try scanning again.';
	}

	function selectFamily(familyId: string) {
		selectedFamilyId = familyId;
		const family = catalogue?.families.find((candidate) => candidate.id === familyId);
		if (family) previewWeight = nearestWeight(family.weights, previewWeight);
	}

	function openFamilyPreview(familyId: string) {
		const family = catalogue?.families.find((candidate) => candidate.id === familyId);
		if (!family) return;

		selectFamily(familyId);
		view = 'preview';
		if (!pinnedFamilyIds.includes(familyId)) {
			pinnedFamilyIds = [...pinnedFamilyIds, familyId];
			savePreferences();
			showToast(`${family.name} added to saved previews.`, 'success');
		}
	}

	function closeFamilyPreview(familyId: string) {
		if (!pinnedFamilyIds.includes(familyId)) return;

		const family = catalogue?.families.find((candidate) => candidate.id === familyId);
		pinnedFamilyIds = pinnedFamilyIds.filter((candidate) => candidate !== familyId);
		if (view === 'preview' && selectedFamilyId === familyId) view = 'library';
		savePreferences();
		showToast(`${family?.name ?? 'Preview'} closed.`, 'success');
	}

	function reorderPinnedFamily(
		draggedFamilyId: string,
		targetFamilyId: string,
		position: ReorderPosition
	) {
		const reordered = reorderIds(pinnedFamilyIds, draggedFamilyId, targetFamilyId, position);
		if (reordered.every((familyId, index) => familyId === pinnedFamilyIds[index])) return;
		pinnedFamilyIds = reordered;
		savePreferences();
	}

	/**
	 * Saving a family is one click from the catalogue row, so it must not move the user:
	 * the list keeps its scroll position and the family lands in the sidebar's saved
	 * previews. Opening the preview is a separate, deliberate action.
	 */
	function toggleFamilyPinned(familyId: string) {
		const family = catalogue?.families.find((candidate) => candidate.id === familyId);
		if (!family) return;

		const isPinned = pinnedFamilyIds.includes(familyId);
		pinnedFamilyIds = isPinned
			? pinnedFamilyIds.filter((candidate) => candidate !== familyId)
			: [...pinnedFamilyIds, familyId];
		savePreferences();
		showToast(
			isPinned
				? `${family.name} removed from saved previews.`
				: `${family.name} added to saved previews.`,
			'success'
		);
	}

	function toggleSelectedFamilyPinned() {
		if (selectedFamily) toggleFamilyPinned(selectedFamily.id);
	}

	function reviewConflict(familyId: string) {
		selectFamily(familyId);
		view = getConflictDestination('review');
	}

	function inspectConflict(familyId: string) {
		selectFamily(familyId);
		view = getConflictDestination('inspect');
	}

	function updateSearch(value: string) {
		search = value;
		displayLimit = PAGE_SIZE;
	}

	function updateGlobalSearch(value: string) {
		view = 'library';
		updateSearch(value);
	}

	function focusSearch() {
		requestAnimationFrame(() => {
			document.querySelector<HTMLInputElement>('[data-font-search]')?.focus();
		});
	}

	function updateFilter(key: LibraryFilterKey, value: string) {
		if (key === 'origin') originFilter = value;
		if (key === 'format') formatFilter = value;
		if (key === 'technology') technologyFilter = value;
		if (key === 'spacing') spacingFilter = value;
		if (key === 'status') statusFilter = value;
		if (key === 'sort') sortOrder = value;
		displayLimit = PAGE_SIZE;
	}

	function clearFilter(key: LibraryFilterKey) {
		updateFilter(key, key === 'sort' ? 'name-asc' : 'all');
	}

	function clearFilters() {
		originFilter = 'all';
		formatFilter = 'all';
		technologyFilter = 'all';
		spacingFilter = 'all';
		statusFilter = 'all';
		displayLimit = PAGE_SIZE;
	}

	function resetAll() {
		search = '';
		clearFilters();
		sortOrder = 'name-asc';
		specimenMode = 'names';
		specimenSize = DEFAULT_SPECIMEN_SIZE;
		specimenWeight = DEFAULT_SPECIMEN_WEIGHT;
	}

	function optionLabel(options: DiscoverFilterOption[], value: string): string {
		return options.find((option) => option.value === value)?.label ?? value;
	}

	function toggleFamily(familyId: string) {
		if (selectedFamilyId === familyId) {
			selectedFamilyId = null;
			return;
		}
		selectFamily(familyId);
	}

	// Local font files reach the web view only after the Rust boundary validates them.
	// The picker returns a trusted path; validate_font_file parses every face and hands
	// back an opaque handle plus a synthetic family, which the preview modal renders.
	async function openPreviewFilePicker() {
		try {
			const validated = await importLocalFontPreview();
			if (!validated) return;
			releaseLocalFontPreview(localPreview?.previewFamily);
			localPreview = validated;
		} catch (error) {
			showToast(commandErrorMessage(error), 'error');
		}
	}

	/**
	 * Asks for font files and shows what FontNest found in them. Nothing is installed here: the
	 * review is a report, and the import only happens if the person says so.
	 */
	async function openImportPicker() {
		try {
			const plan = await reviewChosenFonts(false);
			if (!plan) return;
			importOutcomes = null;
			importPlan = plan;
		} catch (error) {
			showToast(commandErrorMessage(error), 'error');
		}
	}

	async function confirmImport() {
		if (!importPlan || importRunning) return;
		importRunning = true;
		try {
			importOutcomes = await importReviewed(importPlan);
			// Fonts the computer now has are fonts the catalogue should know about.
			if (importOutcomes.some((outcome) => outcome.installed)) await refreshCatalogue();
		} catch (error) {
			showToast(commandErrorMessage(error), 'error');
			closeImport();
		} finally {
			importRunning = false;
		}
	}

	/**
	 * Reads what FontNest has installed and set aside. Read-only and cheap, so it runs whenever
	 * Settings opens rather than being cached into staleness.
	 */
	async function refreshManagedInventory() {
		if (catalogueMode !== 'native') return;
		try {
			managedInventory = await managedFontInventory();
		} catch (error) {
			showToast(commandErrorMessage(error), 'error');
		}
	}

	async function removeManagedFontById(id: string) {
		if (managedBusyId) return;
		managedBusyId = id;
		try {
			const report = await removeManagedFont(id);
			if (report.refused) {
				showToast(refusalDetail(report.refused.reason), 'error');
			} else if (report.removed) {
				showToast('Font removed and set aside.', 'success');
			}
			await refreshManagedInventory();
			// The font is out of service, so the catalogue should stop listing it.
			if (report.removed) await refreshCatalogue();
		} catch (error) {
			showToast(commandErrorMessage(error), 'error');
		} finally {
			managedBusyId = null;
		}
	}

	async function restoreManagedFontById(id: string) {
		if (managedBusyId) return;
		managedBusyId = id;
		try {
			await restoreManagedFont(id);
			showToast('Font put back.', 'success');
			await refreshManagedInventory();
			await refreshCatalogue();
		} catch (error) {
			showToast(commandErrorMessage(error), 'error');
		} finally {
			managedBusyId = null;
		}
	}

	async function discardManagedFontById(id: string) {
		if (managedBusyId) return;
		managedBusyId = id;
		try {
			const freed = await discardManagedFont(id);
			showToast(`Deleted for good. ${formatBytes(freed)} freed.`, 'success');
			await refreshManagedInventory();
		} catch (error) {
			showToast(commandErrorMessage(error), 'error');
		} finally {
			managedBusyId = null;
		}
	}

	function closeImport() {
		if (importRunning) return;
		importPlan = null;
		importOutcomes = null;
	}

	function closeLocalPreview() {
		releaseLocalFontPreview(localPreview?.previewFamily);
		localPreview = null;
	}

	function copyValue(label: string, value: string) {
		void writeClipboardText(value).then((copied) => {
			if (copied) showToast(`${label} copied.`, 'success');
			else showToast('FontNest could not reach the clipboard.', 'error');
		});
	}

	// Font files are reachable only through the backend: the web view holds an opaque face
	// ID, and the Rust side is what turns it back into a path.
	async function revealFaceFile(faceId: string) {
		try {
			// Windows presents its own font directory as a control panel rather than a
			// folder, and files inside it cannot be selected, so system fonts open the
			// folder instead. Say so rather than leaving the user hunting for a highlight.
			if (!(await revealFontFaceFile(faceId))) {
				showToast(
					'Windows does not allow selecting files inside its Fonts folder, so FontNest opened the folder itself.',
					'success'
				);
			}
		} catch (error) {
			showToast(commandErrorMessage(error), 'error');
		}
	}

	async function copyFaceFilePath(faceId: string) {
		try {
			copyValue('File path', await fontFaceFilePath(faceId));
		} catch (error) {
			showToast(commandErrorMessage(error), 'error');
		}
	}

	function familyMenu(family: FontFamilySummary) {
		const firstFaceId = family.faces[0]?.id;
		return familyContextMenu({
			family,
			expanded: selectedFamilyId === family.id,
			pinned: pinnedFamilyIds.includes(family.id),
			native: catalogueMode === 'native',
			onToggleExpanded: () => toggleFamily(family.id),
			onOpenPreview: () => openFamilyPreview(family.id),
			onClosePreview: () => closeFamilyPreview(family.id),
			onReviewConflict: () => reviewConflict(family.id),
			onUseAsPreviewText: () => {
				setPreviewText(family.name);
				specimenMode = 'custom';
				showToast('Preview text updated.', 'success');
			},
			onRevealFile: () => firstFaceId && void revealFaceFile(firstFaceId),
			onCopyFilePath: () => firstFaceId && void copyFaceFilePath(firstFaceId),
			onCopy: copyValue
		});
	}

	function faceMenu(family: FontFamilySummary, face: FontFamilySummary['faces'][number]) {
		return faceContextMenu({
			familyName: family.name,
			face,
			native: catalogueMode === 'native',
			onRevealFile: () => void revealFaceFile(face.id),
			onCopyFilePath: () => void copyFaceFilePath(face.id),
			onCopy: copyValue
		});
	}

	function showToast(message: string, tone: Toast['tone']) {
		if (toastTimer) clearTimeout(toastTimer);
		toast = { message, tone };
		toastTimer = setTimeout(() => {
			toast = null;
		}, 5000);
	}
</script>

<svelte:head>
	<title>FontNest — Working type archive</title>
	<meta
		name="description"
		content="Browse, preview, and inspect the fonts installed on your computer."
	/>
</svelte:head>

<a class="skip-link" href="#main-content">Skip to font catalogue</a>

<AppTitleBar
	{search}
	{loading}
	{theme}
	settingsActive={view === 'settings'}
	{sidebarCollapsed}
	onSearch={updateGlobalSearch}
	onNavigate={navigate}
	onToggleTheme={toggleTheme}
	onToggleSidebar={toggleSidebar}
	onRefresh={() => void refreshCatalogue()}
	onPreview={openPreviewFilePicker}
/>

<div
	class:compact={density === 'compact'}
	class:sidebar-collapsed={sidebarCollapsed}
	class="app-shell"
>
	<AppNavigation
		{view}
		familyCount={catalogue?.familyCount ?? 0}
		conflictCount={catalogue?.conflictCount ?? 0}
		{loading}
		collapsed={sidebarCollapsed}
		{pinnedFamilies}
		{unseenRelease}
		activeFamilyId={view === 'preview' ? (selectedFamily?.id ?? null) : null}
		onNavigate={navigate}
		onOpenPreview={openFamilyPreview}
		onClosePreview={closeFamilyPreview}
		onReorderPreview={reorderPinnedFamily}
		onToggle={toggleSidebar}
		onRefresh={() => void refreshCatalogue()}
		onCopy={copyValue}
	/>

	<main id="main-content">
		{#if view === 'library'}
			<LibraryView
				{catalogue}
				{catalogueMode}
				{loading}
				{errorMessage}
				{prefersReducedMotion}
				{density}
				pageSize={PAGE_SIZE}
				{previewText}
				{pinnedFamilyIds}
				{selectedFamilyId}
				{filterGroups}
				{activeFilters}
				{filteredFamilies}
				{hasResettableState}
				{sortOrder}
				sortOptions={SORT_OPTIONS}
				bind:search
				bind:specimenMode
				bind:specimenSize
				bind:specimenWeight
				bind:displayLimit
				bind:libraryScrollElement
				onClearFilter={clearFilter}
				onClearFilters={clearFilters}
				onResetAll={resetAll}
				onUpdateFilter={updateFilter}
				onUpdateSearch={updateSearch}
				onSetPreviewText={setPreviewText}
				onRefreshCatalogue={refreshCatalogue}
				onImport={openImportPicker}
				onPreviewFile={openPreviewFilePicker}
				onOpenFamilyPreview={openFamilyPreview}
				onToggleFamily={toggleFamily}
				onTogglePinned={toggleFamilyPinned}
				onReviewConflict={reviewConflict}
				{familyMenu}
				{faceMenu}
			/>
		{:else if view === 'discover'}
			<DiscoverView
				installedFamilyNames={catalogue?.families.map((family) => family.name) ?? []}
				{previewText}
				onPreviewText={setPreviewText}
				onInstalled={refreshCatalogue}
				onToast={showToast}
			/>
		{:else if view === 'duplicates'}
			<ConflictsView families={conflictFamilies} onInspect={inspectConflict} />
		{:else if view === 'preview'}
			<FontPreviewView
				family={selectedFamily}
				{previewText}
				{previewSize}
				{previewWeight}
				pinned={selectedFamily ? pinnedFamilyIds.includes(selectedFamily.id) : false}
				native={catalogueMode === 'native'}
				onBack={() => (view = 'library')}
				onTogglePinned={toggleSelectedFamilyPinned}
				onPreviewText={setPreviewText}
				onPreviewSize={(value) => (previewSize = value)}
				onPreviewWeight={(value) => (previewWeight = value)}
				onCopy={copyValue}
				onRevealFile={(faceId) => void revealFaceFile(faceId)}
				onCopyFilePath={(faceId) => void copyFaceFilePath(faceId)}
			/>
		{:else if view === 'whatsNew'}
			<PatchNotesView />
		{:else}
			<SettingsView
				{theme}
				{density}
				{focusOutlines}
				{previewText}
				onTheme={setTheme}
				onDensity={setDensity}
				onFocusOutlines={setFocusOutlines}
				onPreviewText={setPreviewText}
				onViewReleaseNotes={() => navigate('whatsNew')}
				inventory={managedInventory}
				busyId={managedBusyId}
				onRemove={removeManagedFontById}
				onRestore={restoreManagedFontById}
				onDiscard={discardManagedFontById}
			/>
		{/if}
	</main>
</div>

<ContextMenu
	{resolvedTheme}
	{sidebarCollapsed}
	onToast={showToast}
	onRefresh={() => void refreshCatalogue()}
	onToggleTheme={toggleTheme}
	onToggleSidebar={toggleSidebar}
	onNavigate={navigate}
	onSearch={updateGlobalSearch}
	onPreviewText={setPreviewText}
/>

{#if toast}
	<div class:error={toast.tone === 'error'} class="toast" role="status" aria-live="polite">
		<Icon name={toast.tone === 'error' ? 'alert' : 'check'} size={17} />
		<span>{toast.message}</span>
		<button type="button" aria-label="Dismiss notification" onclick={() => (toast = null)}>
			<Icon name="close" size={16} />
		</button>
	</div>
{/if}

{#if localPreview}
	<LocalFontPreview font={localPreview} {previewText} onClose={closeLocalPreview} />
{/if}

{#if importPlan}
	<FontImportReview
		plan={importPlan}
		outcomes={importOutcomes}
		importing={importRunning}
		onConfirm={confirmImport}
		onClose={closeImport}
	/>
{/if}

<style>
	.app-shell {
		--titlebar-height: 48px;
		--app-content-height: calc(100dvh - var(--titlebar-height));

		display: grid;
		height: var(--app-content-height);
		grid-template-columns: 208px minmax(0, 1fr);
		min-height: 0;
		overflow: hidden;
		color: var(--color-text);
		background: var(--color-bg);
		transition: grid-template-columns var(--motion-standard);
	}

	.app-shell.sidebar-collapsed {
		grid-template-columns: 56px minmax(0, 1fr);
	}

	main {
		min-width: 0;
		min-height: 0;
		overflow: auto;
	}

	.toast {
		position: fixed;
		right: 20px;
		bottom: 20px;
		z-index: var(--z-toast);
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		max-width: min(440px, calc(100vw - 32px));
		align-items: center;
		gap: 10px;
		padding: 12px 12px 12px 14px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		color: var(--color-success);
		background: var(--color-raised);
		box-shadow: var(--shadow-floating);
		font-size: var(--text-body-sm);
		animation: toast-in var(--motion-standard);
	}

	.toast.error {
		color: var(--color-danger);
	}

	.toast span {
		color: var(--color-text);
	}

	.toast button {
		display: grid;
		width: 32px;
		height: 32px;
		place-items: center;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		cursor: pointer;
	}

	.toast button:hover {
		color: var(--color-text);
		background: var(--color-selected);
	}

	@keyframes toast-in {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}

	/* A narrow window keeps its sidebar; it just becomes an icon rail. Folding it into a
	   horizontal strip above the content reads as a different app every time you resize. */
	@media (max-width: 819px) {
		.app-shell,
		.app-shell.sidebar-collapsed {
			grid-template-columns: 56px minmax(0, 1fr);
		}
	}

	@media (max-width: 700px) {
		.toast {
			right: 16px;
			bottom: 16px;
			left: 16px;
		}
	}
</style>
