<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { onMount, tick } from 'svelte';
	import { quintOut } from 'svelte/easing';
	import { slide } from 'svelte/transition';

	import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
	import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
	import type { ImportOutcome } from '$lib/bindings/ImportOutcome';
	import type { ImportPlan } from '$lib/bindings/ImportPlan';
	import type { FontOrigin } from '$lib/bindings/FontOrigin';
	import type { ValidatedLocalFont } from '$lib/bindings/ValidatedLocalFont';
	import { checkForUpdates } from '$lib/app-updater';
	import { getConflictDestination } from '$lib/conflict-navigation';
	import AppNavigation, { type AppView } from '$lib/components/AppNavigation.svelte';
	import AppTitleBar from '$lib/components/AppTitleBar.svelte';
	import ConflictsView from '$lib/components/ConflictsView.svelte';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import FontImportReview from '$lib/components/FontImportReview.svelte';
	import LocalFontPreview from '$lib/components/LocalFontPreview.svelte';
	import DiscoverFilterMenu, {
		type DiscoverFilterOption
	} from '$lib/components/DiscoverFilterMenu.svelte';
	import DiscoverView from '$lib/components/DiscoverView.svelte';
	import FilterPopover, { type FilterGroup } from '$lib/components/FilterPopover.svelte';
	import FontPreviewView from '$lib/components/FontPreviewView.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import PatchNotesView from '$lib/components/PatchNotesView.svelte';
	import RangeSlider from '$lib/components/RangeSlider.svelte';
	import SettingsView, {
		type DensityPreference,
		type ThemePreference
	} from '$lib/components/SettingsView.svelte';
	import { createBrowserCatalogue } from '$lib/catalogue/browser-catalogue';
	import { contextMenu } from '$lib/context-menu/action';
	import { writeClipboardText } from '$lib/context-menu/clipboard';
	import { faceContextMenu, familyContextMenu } from '$lib/context-menu/entries';
	import {
		familyOrigin,
		FONT_ORIGIN_ORDER,
		fontOrigin,
		isSystemOnly
	} from '$lib/fonts/font-origin';
	import { importReviewed, reviewChosenFonts } from '$lib/fonts/import';
	import { nearestWeight, weightName } from '$lib/fonts/weights';
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
	import { isStickySurfaceElevated } from '$lib/sticky-surface';
	import {
		discardManagedFont,
		managedFontInventory,
		removeManagedFont,
		restoreManagedFont
	} from '$lib/tauri/commands';
	import { fontFaceFilePath, revealFontFaceFile, scanInstalledFonts } from '$lib/tauri/commands';

	const PAGE_SIZE = 120;
	const SKELETON_ROWS = [0, 1, 2, 3];
	const DEFAULT_PREVIEW = 'What is life but a fevered dream';
	const DEFAULT_SPECIMEN_SIZE = 96;
	const DEFAULT_SPECIMEN_WEIGHT = 400;
	const WEIGHT_NOTE_LINGER_MS = 2600;
	const MAX_DETAIL_FACES = 12;
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
	type LibraryFilterKey = 'origin' | 'format' | 'technology' | 'spacing' | 'status' | 'sort';
	type ActiveLibraryFilter = { key: LibraryFilterKey; label: string };
	type Toast = { message: string; tone: 'success' | 'error' };

	let view = $state<AppView>('library');
	let unseenRelease = $state(false);
	let libraryControlsElement = $state<HTMLElement>();
	let libraryControlsElevated = $state(false);

	function navigate(nextView: AppView) {
		if (nextView === 'whatsNew') unseenRelease = false;
		view = nextView;
	}

	$effect(() => {
		if (view !== 'library') libraryControlsElevated = false;
	});

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
	let weightNotesVisible = $state(false);
	let weightNoteTimer: ReturnType<typeof setTimeout> | undefined;
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

	let formatOptions = $derived.by<DiscoverFilterOption[]>(() => {
		const values = [
			...new Set(catalogue?.families.flatMap((family) => family.formats) ?? [])
		].sort();
		return [
			{ value: 'all', label: 'All formats' },
			...values.map((value) => ({ value, label: value }))
		];
	});

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

	let filteredFamilies = $derived.by(() => {
		const terms = search
			.toLocaleLowerCase()
			.split(/\s+/)
			.map((term) => term.trim())
			.filter(Boolean);

		const matching = (catalogue?.families ?? []).filter((family) => {
			const searchable = [
				family.name,
				...family.styles,
				...family.origins.map((origin) => fontOrigin(origin).label),
				...family.formats,
				family.variable ? 'Variable' : 'Static'
			]
				.join(' ')
				.toLocaleLowerCase();
			return (
				terms.every((term) => searchable.includes(term)) &&
				(originFilter === 'all' || family.origins.includes(originFilter as FontOrigin)) &&
				(formatFilter === 'all' || family.formats.includes(formatFilter)) &&
				(technologyFilter === 'all' ||
					family.variable === (technologyFilter === 'variable')) &&
				(spacingFilter === 'all' ||
					family.monospaced === (spacingFilter === 'monospaced')) &&
				(statusFilter === 'all' || family.hasConflict)
			);
		});

		return matching.sort((left, right) => {
			if (sortOrder === 'name-desc') return right.name.localeCompare(left.name);
			if (sortOrder === 'styles')
				return right.faceCount - left.faceCount || left.name.localeCompare(right.name);
			if (sortOrder === 'faces')
				return right.fileCount - left.fileCount || left.name.localeCompare(right.name);
			return left.name.localeCompare(right.name);
		});
	});

	let renderedFamilies = $derived(filteredFamilies.slice(0, displayLimit));
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

	function handleLibraryScroll(event: Event) {
		const scrollContainer = event.currentTarget as HTMLElement;
		const elevated = isStickySurfaceElevated(
			scrollContainer.scrollTop,
			libraryControlsElement?.offsetTop ?? Number.POSITIVE_INFINITY
		);
		if (elevated !== libraryControlsElevated) libraryControlsElevated = elevated;
	}

	function handleRowKeydown(event: KeyboardEvent, index: number) {
		if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
		event.preventDefault();

		let nextIndex = index;
		if (event.key === 'ArrowDown') nextIndex = Math.min(index + 1, renderedFamilies.length - 1);
		if (event.key === 'ArrowUp') nextIndex = Math.max(index - 1, 0);
		if (event.key === 'Home') nextIndex = 0;
		if (event.key === 'End') nextIndex = renderedFamilies.length - 1;

		const next = renderedFamilies[nextIndex];
		if (!next) return;
		document.querySelectorAll<HTMLButtonElement>('.specimen-toggle')[nextIndex]?.focus();
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

	function safeFontStack(name: string): string {
		return `"${name.replace(/["\\;\n\r]/g, '')}", system-ui, sans-serif`;
	}

	// A variable family covers a continuous range from one file, and the weights the
	// catalogue lists for it are only the named instances it ships (often just 400).
	// Snapping to those would pin the whole slider to one weight, so the requested weight
	// goes straight through and the font's own wght axis clamps it.
	function drawnWeight(family: FontFamilySummary): number {
		return family.variable ? specimenWeight : nearestWeight(family.weights, specimenWeight);
	}

	function familyPreviewStyle(family: FontFamilySummary): string {
		return `font-family: ${safeFontStack(family.name)}; font-weight: ${drawnWeight(family)};`;
	}

	/**
	 * Names the weight a family is actually drawn at when it owns no style at the weight the
	 * slider asks for, so a row that cannot follow the slider says why rather than looking stuck.
	 */
	function substituteWeightNote(family: FontFamilySummary): string | null {
		const drawn = drawnWeight(family);
		return drawn === specimenWeight ? null : `Closest cut: ${weightName(drawn)}`;
	}

	/**
	 * The note answers a question the user just asked by moving the slider, so it shows itself
	 * then gets out of the way rather than sitting on every row that cannot follow the weight.
	 */
	function showWeightNotes() {
		weightNotesVisible = true;
		if (weightNoteTimer) clearTimeout(weightNoteTimer);
		weightNoteTimer = setTimeout(() => (weightNotesVisible = false), WEIGHT_NOTE_LINGER_MS);
	}

	function faceSpecimenStyle(family: FontFamilySummary, weight: number, style: string): string {
		return `font-family: ${safeFontStack(family.name)}; font-weight: ${weight}; font-style: ${style === 'italic' ? 'italic' : 'normal'};`;
	}

	function specimenText(family: FontFamilySummary): string {
		return specimenMode === 'names' ? family.name : previewText.trim() || family.name;
	}

	/**
	 * The collapsed row already names origin, formats, style count, spacing, and technology,
	 * so the open panel says only what the row cannot: how far the family's weights reach.
	 */
	function weightRange(family: FontFamilySummary): string | null {
		if (!family.weights.length) return null;
		const lowest = Math.min(...family.weights);
		const highest = Math.max(...family.weights);
		if (family.variable) return `Weights ${lowest}–${highest} (variable)`;
		return lowest === highest
			? `Weight ${weightName(lowest)} ${lowest}`
			: `Weights ${lowest}–${highest}`;
	}

	function faceDetail(
		family: FontFamilySummary,
		face: FontFamilySummary['faces'][number]
	): string {
		const parts = [face.fileName];
		if (family.origins.length > 1) parts.push(fontOrigin(face.origin).label);
		if (face.variable) parts.push('Variable');
		return parts.join(' · ');
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
			<section
				class="library-view"
				aria-labelledby="library-title"
				bind:this={libraryScrollElement}
				onscroll={handleLibraryScroll}
			>
				<header class="library-header">
					<div class="header-lead">
						<h1 id="library-title">Your fonts</h1>
						<p class="catalogue-summary">
							{#if catalogue}
								{catalogue.familyCount.toLocaleString()} families · {catalogue.faceCount.toLocaleString()}
								faces{#if catalogueMode === 'native'}
									· scanned in {catalogue.scanDurationMs.toLocaleString()}
									ms{/if}
							{:else if loading}
								Reading the installed font catalogue…
							{:else}
								Catalogue unavailable
							{/if}
						</p>
					</div>
					<div class="header-actions">
						<button type="button" class="secondary-action" onclick={openImportPicker}>
							<Icon name="plus" size={16} />
							<span>Import fonts</span>
						</button>
						<button
							type="button"
							class="primary-action"
							onclick={openPreviewFilePicker}
						>
							<svg
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="1.7"
								stroke-linecap="round"
								stroke-linejoin="round"
								aria-hidden="true"
								width="16"
								height="16"
							>
								<path d="M12 15.5V4.75M8.25 8.5 12 4.75l3.75 3.75" />
								<path
									d="M5 14.5v3.25A1.75 1.75 0 0 0 6.75 19.5h10.5A1.75 1.75 0 0 0 19 17.75V14.5"
								/>
							</svg>
							<span>Preview a font</span>
						</button>
					</div>
				</header>

				<section
					bind:this={libraryControlsElement}
					class:is-elevated={libraryControlsElevated}
					class="library-controls sticky-control-surface"
					aria-label="Library controls"
				>
					<div class="primary-toolbar">
						<label class="search-control">
							<span class="sr-only">Search</span>
							<Icon name="search" size={15} />
							<input
								data-font-search
								type="search"
								placeholder="Families, styles, origins"
								value={search}
								oninput={(event) => updateSearch(event.currentTarget.value)}
							/>
						</label>
						<div class="filter-strip">
							<FilterPopover
								id="library-filters"
								groups={filterGroups}
								onChange={(key, value) =>
									updateFilter(key as LibraryFilterKey, value)}
								onClear={clearFilters}
							/>
							<DiscoverFilterMenu
								id="library-sort"
								label="Sort"
								value={sortOrder}
								options={SORT_OPTIONS}
								onChange={(value) => updateFilter('sort', value)}
							/>
							<div
								class:empty={activeFilters.length === 0}
								class="active-filter-summary"
								aria-live="polite"
							>
								{#each activeFilters as filter (filter.key)}
									<button
										type="button"
										aria-label={`Remove ${filter.label} filter`}
										onclick={() => clearFilter(filter.key)}
									>
										{filter.label}<Icon name="close" size={12} />
									</button>
								{/each}
							</div>
							<button
								type="button"
								class="reset-action"
								disabled={!hasResettableState}
								onclick={resetAll}>Reset all</button
							>
						</div>
					</div>

					<div class="specimen-toolbar">
						<label class="preview-text-control">
							<span class="sr-only">Preview text</span>
							<Icon name="font" size={15} />
							<input
								type="text"
								value={previewText}
								placeholder="Type a shared specimen"
								disabled={specimenMode === 'names'}
								oninput={(event) => setPreviewText(event.currentTarget.value)}
							/>
						</label>
						<div class="specimen-modes" role="group" aria-label="Specimen text mode">
							<button
								type="button"
								class:active={specimenMode === 'names'}
								aria-pressed={specimenMode === 'names'}
								onclick={() => (specimenMode = 'names')}>Names</button
							>
							<button
								type="button"
								class:active={specimenMode === 'custom'}
								aria-pressed={specimenMode === 'custom'}
								onclick={() => (specimenMode = 'custom')}>Your text</button
							>
						</div>
						<RangeSlider
							label="Size"
							value={specimenSize}
							min={48}
							max={148}
							step={4}
							display={`${specimenSize}px`}
							onChange={(value) => (specimenSize = value)}
						/>
						<RangeSlider
							label="Weight"
							value={specimenWeight}
							min={100}
							max={900}
							step={100}
							display={weightName(specimenWeight)}
							valueText={`${weightName(specimenWeight)} ${specimenWeight}`}
							valueWidth="68px"
							onChange={(value) => {
								specimenWeight = value;
								showWeightNotes();
							}}
						/>
					</div>
				</section>

				<div class="specimen-feed" style={`--specimen-size: ${specimenSize}px`}>
					<div class="catalogue-heading">
						<strong>{filteredFamilies.length.toLocaleString()} families</strong>
						<span>Rendered in the fonts installed on this computer</span>
					</div>

					{#if loading && !catalogue}
						<div class="specimen-list" aria-label="Loading font families">
							{#each SKELETON_ROWS as row (row)}
								<div class="specimen-entry loading-entry" aria-hidden="true">
									<div class="loading-meta">
										<span></span><span></span><span></span>
									</div>
									<div class="specimen-skeleton">
										<span></span><span></span><span></span>
									</div>
								</div>
							{/each}
						</div>
					{:else if errorMessage}
						<div class="catalogue-state" role="alert">
							<div class="state-icon error"><Icon name="alert" size={20} /></div>
							<h2>Catalogue scan did not finish</h2>
							<p>{errorMessage}</p>
							<button type="button" onclick={() => void refreshCatalogue()}
								>Scan again</button
							>
						</div>
					{:else if !catalogue?.familyCount}
						<div class="catalogue-state">
							<div class="state-icon"><Icon name="font" size={20} /></div>
							<h2 class="type-display">No installed fonts found</h2>
							<p>
								Scan again, or open a font file to preview it without installing
								anything.
							</p>
							<button type="button" onclick={openPreviewFilePicker}
								>Preview a font file</button
							>
						</div>
					{:else if !renderedFamilies.length}
						<div class="catalogue-state">
							<div class="state-icon"><Icon name="search" size={20} /></div>
							<h2>No families match</h2>
							<p>Try a shorter search, or remove one of the active filters.</p>
							<button
								type="button"
								onclick={() => {
									search = '';
									clearFilters();
								}}>Clear search and filters</button
							>
						</div>
					{:else}
						<div class="specimen-list" aria-label="Font families">
							{#each renderedFamilies as family, index (family.id)}
								{@const saved = pinnedFamilyIds.includes(family.id)}
								<article
									use:contextMenu={() => familyMenu(family)}
									class:selected={selectedFamilyId === family.id}
									class="specimen-entry"
								>
									<button
										type="button"
										class="specimen-toggle"
										aria-expanded={selectedFamilyId === family.id}
										aria-controls={`family-details-${family.id}`}
										onclick={() => toggleFamily(family.id)}
										onkeydown={(event) => handleRowKeydown(event, index)}
									>
										<span class="family-line">
											<strong>{family.name}</strong>
											<span
												class="meta-origin"
												class:is-added={!isSystemOnly(family.origins)}
												title={familyOrigin(family.origins).description}
												>{familyOrigin(family.origins).label}</span
											>
											<span class="meta-format"
												>{family.formats.join(' · ')}</span
											>
											<span class="meta-count">
												{family.faceCount}
												{family.faceCount === 1 ? 'style' : 'styles'}
											</span>
											<span class="meta-spacing"
												>{family.monospaced
													? 'Monospaced'
													: 'Proportional'}</span
											>
											{#if family.variable}
												<span
													class="meta-variable"
													title="One file covers a range of weights or widths."
													>Variable</span
												>
											{/if}
											{#if family.hasConflict}
												<span class="conflict-label"
													><Icon name="alert" size={12} /> Conflict</span
												>
											{/if}
											<span class="open-label">
												{selectedFamilyId === family.id
													? 'Close'
													: 'Open family'}
												<Icon name="chevron" size={13} />
											</span>
										</span>
										<span
											class="specimen-canvas"
											style={familyPreviewStyle(family)}
										>
											<span class="specimen-text">{specimenText(family)}</span
											>
											{#if substituteWeightNote(family)}
												<small class:visible={weightNotesVisible}
													>{substituteWeightNote(family)}</small
												>
											{/if}
										</span>
									</button>

									<!-- Outside the disclosure button on purpose: saving a family and
									     opening it are different intents, and a button inside a button
									     is invalid markup. -->
									<button
										type="button"
										class:saved
										class="row-save"
										aria-pressed={saved}
										aria-label={saved
											? `Remove ${family.name} from saved previews`
											: `Save ${family.name} to previews`}
										title={saved ? 'Saved to previews' : 'Save to previews'}
										onclick={() => toggleFamilyPinned(family.id)}
									>
										<Icon name={saved ? 'check' : 'plus'} size={15} />
									</button>

									{#if selectedFamilyId === family.id}
										<section
											id={`family-details-${family.id}`}
											class="family-details"
											in:slide={{
												duration: prefersReducedMotion ? 0 : 240,
												easing: quintOut
											}}
											out:slide={{
												duration: prefersReducedMotion ? 0 : 170,
												easing: quintOut
											}}
										>
											<div class="detail-bar">
												<p class="detail-facts">
													{#if weightRange(family)}
														<span>{weightRange(family)}</span>
													{/if}
													<span
														>{family.fileCount}
														{family.fileCount === 1
															? 'file'
															: 'files'}</span
													>
													<span
														>{familyOrigin(family.origins)
															.description}</span
													>
												</p>
												<div class="detail-actions">
													{#if family.hasConflict}
														<button
															type="button"
															class="detail-action ghost"
															onclick={() =>
																reviewConflict(family.id)}
														>
															<Icon name="alert" size={15} /> Review conflict
														</button>
													{/if}
													<button
														type="button"
														class="detail-action"
														onclick={() => openFamilyPreview(family.id)}
													>
														Open preview
														<Icon name="chevron" size={14} />
													</button>
												</div>
											</div>

											<!-- Every face draws the same string so the cuts stack and compare;
											     setting each one in its own style name compared different words. -->
											<ul class="face-list">
												{#each family.faces.slice(0, MAX_DETAIL_FACES) as face (face.id)}
													<li
														use:contextMenu={() =>
															faceMenu(family, face)}
													>
														<span class="face-meta">
															<span class="face-name">
																<strong>{face.styleName}</strong>
																<span class="face-weight"
																	>{face.weight}</span
																>
															</span>
															<small title={face.fileName}
																>{faceDetail(family, face)}</small
															>
														</span>
														<span
															class="face-specimen"
															style={faceSpecimenStyle(
																family,
																face.weight,
																face.style
															)}>{specimenText(family)}</span
														>
													</li>
												{/each}
											</ul>
											{#if family.faces.length > MAX_DETAIL_FACES}
												<button
													type="button"
													class="more-faces"
													onclick={() => openFamilyPreview(family.id)}
												>
													Show all {family.faceCount} styles in preview
												</button>
											{/if}
										</section>
									{/if}
								</article>
							{/each}
						</div>

						{#if renderedFamilies.length < filteredFamilies.length}
							<div class="load-more-row">
								<button type="button" onclick={() => (displayLimit += PAGE_SIZE)}>
									{Math.min(
										PAGE_SIZE,
										filteredFamilies.length - renderedFamilies.length
									)} more
								</button>
								<span
									>{renderedFamilies.length.toLocaleString()} of {filteredFamilies.length.toLocaleString()}</span
								>
							</div>
						{/if}
					{/if}
				</div>
			</section>
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

	.library-view {
		display: flex;
		width: 100%;
		min-width: 0;
		height: 100%;
		min-height: 0;
		flex-direction: column;
		overflow-x: hidden;
		overflow-y: auto;
		background: var(--color-surface);
	}

	.library-header {
		display: flex;
		width: 100%;
		min-width: 0;
		flex: none;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2xl);
		padding: 18px 24px 14px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface);
	}

	.header-lead {
		min-width: 0;
	}

	h1,
	h2,
	p {
		margin-top: 0;
	}

	.library-header h1 {
		margin: 3px 0 0;
		font-size: var(--text-heading);
		line-height: 1.15;
		letter-spacing: -0.03em;
		text-wrap: balance;
	}

	.catalogue-summary {
		margin: 5px 0 0;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
	}

	.header-actions {
		display: flex;
		flex: none;
		gap: var(--space-sm);
	}

	.primary-action {
		display: inline-flex;
		height: 36px;
		align-items: center;
		justify-content: center;
		gap: 7px;
		padding: 0 12px;
		border: 1px solid var(--color-accent);
		border-radius: var(--radius-md);
		color: var(--color-accent-ink);
		background: var(--color-accent);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
		transition:
			background var(--motion-fast),
			transform var(--motion-fast);
	}

	.primary-action:hover {
		background: var(--color-accent-hover);
	}

	.primary-action:active {
		transform: translateY(1px);
	}

	/* Importing is the quieter of the two: previewing a file changes nothing, and installing one
	   should not be the louder invitation of the pair. */
	.secondary-action {
		display: inline-flex;
		height: 36px;
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
		transition:
			background var(--motion-fast),
			transform var(--motion-fast);
	}

	.secondary-action:hover {
		background: var(--color-hover);
	}

	.secondary-action:active {
		transform: translateY(1px);
	}

	/* Controls — shared vocabulary with the Discover view */
	.library-controls {
		position: sticky;
		top: 0;
		z-index: var(--z-sticky);
		width: 100%;
		min-width: 0;
		flex: none;
	}

	.primary-toolbar {
		display: flex;
		width: 100%;
		min-width: 0;
		align-items: center;
		gap: var(--space-md);
		padding: 10px 24px;
		border-bottom: 1px solid var(--color-border);
	}

	.search-control,
	.preview-text-control {
		display: grid;
		min-width: 0;
		grid-template-columns: auto minmax(0, 1fr);
		align-items: center;
		gap: var(--space-sm);
		padding-left: 10px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-subtle);
		background: var(--color-control);
		font-size: var(--text-micro);
	}

	.search-control:focus-within,
	.preview-text-control:focus-within {
		border-color: var(--focus-ring-border);
	}

	.search-control:has(input:focus-visible),
	.preview-text-control:has(input:focus-visible) {
		outline: 2px solid var(--focus-ring);
		outline-offset: 2px;
	}

	.search-control input,
	.preview-text-control input {
		width: 100%;
		height: 38px;
		min-width: 0;
		border: 0;
		outline: 0;
		color: var(--color-text);
		background: transparent;
		font-size: var(--text-label);
	}

	.search-control input::placeholder,
	.preview-text-control input::placeholder {
		color: var(--color-muted);
	}

	.preview-text-control input:disabled {
		color: var(--color-subtle);
	}

	.search-control {
		flex: 0 1 380px;
	}

	.filter-strip {
		display: flex;
		min-width: 0;
		flex: 1 1 auto;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-sm);
	}

	.filter-strip :global(.filter-control) {
		min-width: 0;
		flex: none;
	}

	.specimen-toolbar {
		display: flex;
		width: 100%;
		min-width: 0;
		align-items: center;
		gap: var(--space-md);
		padding: 10px 24px;
		border-bottom: 1px solid var(--color-border);
	}

	.preview-text-control {
		width: min(340px, 30vw);
		flex: none;
	}

	.specimen-modes {
		display: inline-flex;
		flex: none;
		padding: 2px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		background: var(--color-control);
	}

	.specimen-modes button {
		height: 32px;
		padding: 0 10px;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.specimen-modes button:hover {
		color: var(--color-text);
	}

	.specimen-modes button.active {
		color: var(--color-text);
		background: var(--color-selected);
	}

	/* The chips take their own line rather than shrink into illegibility: they are the
	   only place an active filter is named now that the trigger just shows a count. */
	.active-filter-summary {
		display: flex;
		min-width: 0;
		flex: 1 1 260px;
		align-items: center;
		gap: 6px;
		overflow-x: auto;
		scrollbar-width: none;
	}

	/* With no chips the region still has to exist for its live announcements, but it must
	   not claim a 260px basis and push Reset all onto a line of its own. */
	.active-filter-summary.empty {
		flex: 0 0 0;
	}

	.active-filter-summary button {
		display: inline-flex;
		height: 28px;
		flex: none;
		align-items: center;
		gap: 5px;
		padding: 0 8px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-shell);
		color: var(--color-muted);
		background: var(--color-control);
		font-size: var(--text-micro);
		cursor: pointer;
	}

	.active-filter-summary button:hover {
		color: var(--color-text);
		border-color: var(--color-subtle);
	}

	.reset-action {
		height: 34px;
		flex: none;
		margin-left: auto;
		padding: 0;
		border: 0;
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.reset-action:hover:not(:disabled) {
		color: var(--color-text);
	}

	.reset-action:disabled {
		opacity: 0.45;
	}

	/* Specimen feed */
	.specimen-feed {
		container-type: inline-size;
		width: 100%;
		min-width: 0;
		min-height: 0;
		flex: none;
		overflow: visible;
		background: var(--color-surface);
	}

	.catalogue-heading {
		display: flex;
		height: 40px;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-lg);
		padding: 0 24px;
		border-bottom: 1px solid var(--color-border);
		color: var(--color-subtle);
		background: var(--color-panel);
		font-size: var(--text-micro);
	}

	.catalogue-heading strong {
		color: var(--color-text);
		font-size: var(--text-label);
		font-variant-numeric: tabular-nums;
	}

	.specimen-entry {
		position: relative;
		content-visibility: auto;
		contain-intrinsic-size: auto 230px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface);
		transition: background var(--motion-fast);
	}

	.specimen-entry.selected {
		content-visibility: visible;
		background: color-mix(in srgb, var(--color-selected) 34%, var(--color-surface));
	}

	.specimen-toggle {
		display: block;
		width: 100%;
		padding: 0;
		border: 0;
		color: var(--color-text);
		background: transparent;
		text-align: left;
		cursor: pointer;
	}

	.specimen-toggle:hover {
		background: color-mix(in srgb, var(--color-hover) 46%, transparent);
	}

	.family-line {
		display: flex;
		min-height: 45px;
		align-items: center;
		gap: clamp(10px, 1.4vw, 22px);
		/* Right padding clears the save button, which sits over the row rather than
		   inside the disclosure button. */
		padding: 0 62px 0 24px;
		overflow: hidden;
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.family-line > strong {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		color: var(--color-text);
		font-size: var(--text-label);
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.family-line > span {
		flex: none;
		white-space: nowrap;
	}

	.meta-origin {
		color: var(--color-subtle);
	}

	/* Most libraries are mostly system fonts, so the fonts somebody installed carry the
	   emphasis and stay findable while scrolling. */
	.meta-origin.is-added {
		color: var(--color-text);
		font-weight: 650;
	}

	.meta-variable {
		color: var(--color-text);
		font-weight: 650;
	}

	.conflict-label {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		color: var(--color-warning);
	}

	.open-label {
		display: inline-flex;
		align-items: center;
		margin-left: auto;
		gap: 6px;
		color: var(--color-muted);
		font-weight: 650;
	}

	.open-label :global(svg) {
		transition: transform var(--motion-fast);
	}

	.selected .open-label :global(svg) {
		transform: rotate(90deg);
	}

	/* Quiet on a list of thousands, but always present: a control that appears only on
	   hover is unreachable by touch and easy to miss by keyboard. */
	.row-save {
		position: absolute;
		top: 7px;
		right: 18px;
		display: inline-flex;
		width: 31px;
		height: 31px;
		align-items: center;
		justify-content: center;
		padding: 0;
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		color: var(--color-subtle);
		background: transparent;
		cursor: pointer;
		transition:
			color var(--motion-fast),
			border-color var(--motion-fast),
			background var(--motion-fast);
	}

	.row-save:hover {
		border-color: var(--color-border);
		color: var(--color-text);
		background: var(--color-control);
	}

	.row-save.saved {
		border-color: color-mix(in srgb, var(--color-accent) 55%, var(--color-border));
		color: var(--color-text);
		background: color-mix(in srgb, var(--color-accent) 12%, var(--color-control));
	}

	.specimen-canvas {
		position: relative;
		display: flex;
		min-height: calc(var(--specimen-size) * 1.42 + 16px);
		align-items: center;
		padding: 14px 24px;
		overflow: hidden;
	}

	/* The canvas carries the family's own font so the specimen inherits it; the note has to
	   opt back out or it would be set in the typeface it is describing. */
	.specimen-canvas > small {
		position: absolute;
		right: 24px;
		bottom: 10px;
		color: var(--color-subtle);
		font-family: Geist, 'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif;
		font-size: var(--text-micro);
		font-weight: 400;
		opacity: 0;
		transition: opacity 420ms cubic-bezier(0.16, 1, 0.3, 1);
		pointer-events: none;
	}

	.specimen-canvas > small.visible {
		opacity: 1;
	}

	@media (prefers-reduced-motion: reduce) {
		.specimen-canvas > small {
			transition: none;
		}

		.specimen-entry,
		.row-save,
		.detail-action,
		.more-faces {
			transition: none;
		}
	}

	.specimen-text {
		display: block;
		max-width: 100%;
		font-size: var(--specimen-size);
		font-kerning: normal;
		font-optical-sizing: auto;
		line-height: 1.14;
		letter-spacing: -0.035em;
		white-space: nowrap;
	}

	.compact .specimen-canvas {
		min-height: calc(var(--specimen-size) * 1.15 + 14px);
		padding-block: 12px;
	}

	.compact .specimen-text {
		font-size: calc(var(--specimen-size) * 0.72);
	}

	/* Loading skeleton */
	.loading-entry {
		padding-bottom: 8px;
	}

	.loading-meta {
		display: flex;
		height: 45px;
		align-items: center;
		gap: 16px;
		padding: 0 24px;
	}

	.loading-meta span {
		width: 110px;
		height: 9px;
		border-radius: var(--radius-xs);
		background: var(--color-skeleton);
	}

	.specimen-skeleton {
		display: flex;
		width: min(900px, 82%);
		align-items: center;
		gap: 10px;
		margin-left: 24px;
	}

	.specimen-skeleton span {
		height: clamp(48px, 6vw, 78px);
		border-radius: var(--radius-sm);
		background: var(--color-skeleton);
		animation: skeleton-pulse 1.25s ease-in-out infinite alternate;
	}

	.specimen-skeleton span:nth-child(1) {
		width: 42%;
	}

	.specimen-skeleton span:nth-child(2) {
		width: 27%;
	}

	.specimen-skeleton span:nth-child(3) {
		width: 18%;
	}

	/* Inline family detail (replaces the sidebar inspector) */
	.family-details {
		padding: 18px 24px 22px;
		border-top: 1px solid var(--color-border);
		background: color-mix(in srgb, var(--color-panel) 62%, transparent);
	}

	/* The collapsed row already carries origin, format, style count, spacing, and
	   technology, so this bar only adds what it cannot fit. */
	.detail-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-xl);
	}

	.detail-facts {
		display: flex;
		min-width: 0;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 4px 10px;
		margin: 0;
		color: var(--color-muted);
		font-size: var(--text-micro);
	}

	.detail-facts span + span::before {
		margin-right: 10px;
		color: var(--color-subtle);
		content: '·';
	}

	.detail-actions {
		display: flex;
		flex: none;
		gap: var(--space-sm);
	}

	.detail-action {
		display: inline-flex;
		height: 34px;
		align-items: center;
		gap: 7px;
		padding: 0 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		white-space: nowrap;
		cursor: pointer;
		transition:
			background var(--motion-fast),
			border-color var(--motion-fast),
			color var(--motion-fast);
	}

	.detail-action:hover {
		background: var(--color-selected);
	}

	.detail-action.ghost {
		color: var(--color-warning);
		background: transparent;
	}

	.detail-action.ghost:hover {
		background: color-mix(in srgb, var(--color-warning) 10%, transparent);
	}

	.face-list {
		display: grid;
		margin: 12px 0 0;
		padding: 0;
		list-style: none;
		border-top: 1px solid var(--color-border);
	}

	.face-list li {
		display: grid;
		min-height: 44px;
		grid-template-columns: minmax(126px, 180px) minmax(0, 1fr);
		align-items: center;
		gap: 20px;
		padding: 7px 0;
		overflow: hidden;
		border-bottom: 1px solid var(--color-border);
	}

	.face-meta {
		display: grid;
		min-width: 0;
		gap: 1px;
	}

	.face-name {
		display: flex;
		align-items: baseline;
		gap: 7px;
	}

	.face-name strong {
		overflow: hidden;
		font-size: var(--text-body-sm);
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.face-weight {
		flex: none;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
	}

	.face-meta small {
		overflow: hidden;
		color: var(--color-subtle);
		font-size: var(--text-micro);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* Smaller than the row specimen above it: this column is a waterfall for comparing
	   cuts, not a second headline. */
	.face-specimen {
		overflow: hidden;
		font-size: clamp(20px, 2.4vw, 32px);
		line-height: 1.25;
		letter-spacing: -0.02em;
		white-space: nowrap;
	}

	.more-faces {
		margin: 12px 0 0;
		padding: 0;
		border: 0;
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-micro);
		font-weight: 650;
		text-decoration: underline;
		text-underline-offset: 3px;
		cursor: pointer;
		transition: color var(--motion-fast);
	}

	.more-faces:hover {
		color: var(--color-text);
	}

	/* Empty / error states */
	.catalogue-state {
		display: grid;
		min-height: 360px;
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
		margin-bottom: 4px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-muted);
		background: var(--color-panel);
	}

	.state-icon.error {
		color: var(--color-danger);
	}

	/* The display-set state brings its own size from .type-display; the rest stay small. */
	.catalogue-state h2:not(.type-display) {
		margin-bottom: 0;
		font-size: var(--text-heading-sm);
	}

	.catalogue-state h2.type-display {
		margin-bottom: 0;
	}

	.catalogue-state p {
		max-width: 48ch;
		margin-bottom: var(--space-sm);
		color: var(--color-muted);
		font-size: var(--text-body-sm);
	}

	.catalogue-state button {
		display: inline-flex;
		min-height: 36px;
		align-items: center;
		justify-content: center;
		padding: 0 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.catalogue-state button:hover {
		background: var(--color-selected);
	}

	.load-more-row {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-md);
		padding: 18px 24px 26px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.load-more-row button {
		display: inline-flex;
		min-height: 36px;
		align-items: center;
		justify-content: center;
		padding: 0 12px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.load-more-row button:hover {
		background: var(--color-selected);
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

	@keyframes skeleton-pulse {
		50% {
			opacity: 0.45;
		}
	}

	@keyframes toast-in {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}

	/* Container queries — collapse meta as the feed narrows */
	@container (max-width: 940px) {
		.meta-format,
		.meta-spacing {
			display: none;
		}

		.specimen-text {
			font-size: min(var(--specimen-size), 13cqi);
		}
	}

	@container (max-width: 680px) {
		.meta-origin,
		.meta-variable,
		.meta-count {
			display: none;
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
		.library-header {
			padding: 16px 16px 12px;
		}

		.primary-action {
			width: 36px;
			padding: 0;
		}

		.primary-action span {
			display: none;
		}

		.primary-toolbar,
		.specimen-toolbar {
			padding-inline: 16px;
		}

		.primary-toolbar {
			flex-wrap: wrap;
			gap: var(--space-sm);
		}

		.search-control {
			flex: 1 1 100%;
		}

		.filter-strip {
			width: 100%;
		}

		.specimen-toolbar {
			flex-wrap: wrap;
		}

		.preview-text-control {
			width: auto;
			flex: 1 1 240px;
		}

		.catalogue-heading,
		.family-line,
		.specimen-canvas,
		.family-details {
			padding-inline: 16px;
		}

		.family-line {
			padding-right: 54px;
		}

		.row-save {
			right: 12px;
		}

		.detail-bar {
			align-items: stretch;
			flex-direction: column;
			gap: var(--space-md);
		}

		.face-list li {
			grid-template-columns: 1fr;
			gap: 4px;
		}

		.toast {
			right: 16px;
			bottom: 16px;
			left: 16px;
		}
	}

	@media (max-width: 520px) {
		.catalogue-summary {
			max-width: 32ch;
		}

		.catalogue-heading > span {
			display: none;
		}
	}

	.library-header {
		display: flex;
		width: 100%;
		min-width: 0;
		flex: none;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2xl);
		padding: 18px 24px 14px;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-surface);
	}

	.header-lead {
		min-width: 0;
	}

	.library-header h1 {
		margin: 0;
		font-size: var(--text-heading);
		line-height: 1.15;
		letter-spacing: -0.03em;
		text-wrap: balance;
	}

	.catalogue-summary {
		margin: 5px 0 0;
		color: var(--color-muted);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
	}

	.header-actions {
		display: flex;
		flex: none;
		gap: var(--space-sm);
	}

	.primary-action {
		display: inline-flex;
		height: 36px;
		align-items: center;
		justify-content: center;
		gap: 7px;
		padding: 0 12px;
		border: 1px solid var(--color-accent);
		border-radius: var(--radius-md);
		color: var(--color-accent-ink);
		background: var(--color-accent);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
		transition:
			background var(--motion-fast),
			transform var(--motion-fast);
	}

	.primary-action:hover {
		background: var(--color-accent-hover);
	}

	.primary-action:active {
		transform: translateY(1px);
	}
</style>
