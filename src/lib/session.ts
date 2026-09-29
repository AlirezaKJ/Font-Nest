import type { AppView } from '$lib/components/AppNavigation.svelte';

/**
 * What FontNest remembers about where you were: the view, the family you had open, what you had
 * filtered and sorted by, and how far down the list you had scrolled.
 *
 * Everything here is read back from storage a person can edit, and from an earlier version of the
 * application, so nothing is trusted. Each field is checked on its own and a value that does not
 * make sense is dropped rather than taking the rest of the session down with it.
 */

/**
 * Views worth reopening. `satisfies` keeps a typo out of the list. A view added to `AppView` and
 * not listed here simply is not restored, which lands on the library: safe, not broken.
 */
const RESTORABLE_VIEWS = [
	'library',
	'discover',
	'duplicates',
	'preview',
	'settings'
] as const satisfies readonly AppView[];

/** Sort orders the library offers. */
const SORT_ORDERS = ['name-asc', 'name-desc', 'styles', 'faces'] as const;

const SPECIMEN_MODES = ['names', 'custom'] as const;

/** Nothing below the first page is worth restoring, and nothing above this is worth rendering. */
const MAX_DISPLAY_LIMIT = 5000;

/** Anything further down than this is a scroll position from a catalogue that no longer exists. */
const MAX_SCROLL_TOP = 5_000_000;

export type LibraryFilters = {
	origin: string;
	format: string;
	technology: string;
	spacing: string;
	status: string;
};

export type Session = {
	view: (typeof RESTORABLE_VIEWS)[number];
	selectedFamilyId: string | null;
	search: string;
	filters: LibraryFilters;
	sortOrder: (typeof SORT_ORDERS)[number];
	specimenMode: (typeof SPECIMEN_MODES)[number];
	specimenSize: number;
	specimenWeight: number;
	previewSize: number;
	previewWeight: number;
	displayLimit: number;
	scrollTop: number;
};

/** The shape family identifiers take, so a stored one can be rejected before it is looked up. */
const FAMILY_ID_PATTERN = /^family:[0-9a-f]{32}$/;

/** A filter value is either "all" or one of the values the catalogue produced. */
const FILTER_VALUE_PATTERN = /^[\w-]{1,64}$/;

function asRecord(value: unknown): Record<string, unknown> {
	return typeof value === 'object' && value !== null ? (value as Record<string, unknown>) : {};
}

function member<T extends string>(value: unknown, allowed: readonly T[]): T | undefined {
	return typeof value === 'string' && (allowed as readonly string[]).includes(value)
		? (value as T)
		: undefined;
}

function boundedNumber(value: unknown, min: number, max: number): number | undefined {
	if (typeof value !== 'number' || !Number.isFinite(value)) return undefined;
	const rounded = Math.round(value);
	return rounded >= min && rounded <= max ? rounded : undefined;
}

function filterValue(value: unknown): string | undefined {
	return typeof value === 'string' && FILTER_VALUE_PATTERN.test(value) ? value : undefined;
}

/**
 * The view to reopen on. What's New is deliberately not one: having read the release notes, you
 * do not want them again every launch, so that session reopens on the library.
 */
export function restorableView(view: AppView): Session['view'] {
	return member(view, RESTORABLE_VIEWS) ?? 'library';
}

/** The sort order to store, falling back to the library's own default. */
export function restorableSortOrder(sortOrder: string): Session['sortOrder'] {
	return member(sortOrder, SORT_ORDERS) ?? 'name-asc';
}

/**
 * Reads a stored session, keeping only the parts that still make sense.
 *
 * Returns a partial session on purpose: a field that was never stored, or that no longer passes,
 * is simply absent, and the caller keeps whatever default it already had.
 */
export function parseSession(raw: unknown): Partial<Session> {
	const stored = asRecord(raw);
	const storedFilters = asRecord(stored.filters);
	const session: Partial<Session> = {};

	const view = member(stored.view, RESTORABLE_VIEWS);
	if (view) session.view = view;

	if (
		typeof stored.selectedFamilyId === 'string' &&
		FAMILY_ID_PATTERN.test(stored.selectedFamilyId)
	) {
		session.selectedFamilyId = stored.selectedFamilyId;
	}

	// A search long enough to be a paste accident is not worth reopening with.
	if (typeof stored.search === 'string' && stored.search.length <= 200) {
		session.search = stored.search;
	}

	const filters: Partial<LibraryFilters> = {};
	for (const key of ['origin', 'format', 'technology', 'spacing', 'status'] as const) {
		const value = filterValue(storedFilters[key]);
		if (value) filters[key] = value;
	}
	if (Object.keys(filters).length) {
		session.filters = {
			origin: 'all',
			format: 'all',
			technology: 'all',
			spacing: 'all',
			status: 'all',
			...filters
		};
	}

	const sortOrder = member(stored.sortOrder, SORT_ORDERS);
	if (sortOrder) session.sortOrder = sortOrder;

	const specimenMode = member(stored.specimenMode, SPECIMEN_MODES);
	if (specimenMode) session.specimenMode = specimenMode;

	const specimenSize = boundedNumber(stored.specimenSize, 8, 400);
	if (specimenSize !== undefined) session.specimenSize = specimenSize;

	const specimenWeight = boundedNumber(stored.specimenWeight, 1, 1000);
	if (specimenWeight !== undefined) session.specimenWeight = specimenWeight;

	const previewSize = boundedNumber(stored.previewSize, 8, 400);
	if (previewSize !== undefined) session.previewSize = previewSize;

	const previewWeight = boundedNumber(stored.previewWeight, 1, 1000);
	if (previewWeight !== undefined) session.previewWeight = previewWeight;

	const displayLimit = boundedNumber(stored.displayLimit, 1, MAX_DISPLAY_LIMIT);
	if (displayLimit !== undefined) session.displayLimit = displayLimit;

	const scrollTop = boundedNumber(stored.scrollTop, 0, MAX_SCROLL_TOP);
	if (scrollTop !== undefined) session.scrollTop = scrollTop;

	return session;
}

/**
 * Settles the session against the catalogue that actually loaded.
 *
 * A family can be uninstalled between one launch and the next, and reopening the preview of a
 * font that is no longer there would be a blank screen with nothing to explain it.
 */
export function reconcileSession(
	session: Partial<Session>,
	familyExists: (familyId: string) => boolean
): Partial<Session> {
	const selectedFamilyId =
		session.selectedFamilyId && familyExists(session.selectedFamilyId)
			? session.selectedFamilyId
			: null;

	const view = session.view === 'preview' && !selectedFamilyId ? 'library' : session.view;

	return { ...session, selectedFamilyId, view };
}
