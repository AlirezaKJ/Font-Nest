/**
 * The settings the library is looked at through: what is searched for, what is filtered out, how
 * it is ordered, and how big the specimens are drawn.
 *
 * These are one thing, not eleven. They were eleven separate pieces of route state, which is why
 * the view that uses them needed a prop for each one and a callback for every way of changing
 * them. The route still persists them, and that is the only reason it ever held them.
 */

import type { Session } from '$lib/session';

import type { SpecimenMode } from './specimen';
import {
	type LibraryFilterKey,
	type LibraryFilters,
	type LibrarySortOrder,
	NO_FILTERS
} from './filters';

/** How much of the list is added each time somebody asks for more. */
export const PAGE_SIZE = 120;
export const DEFAULT_SPECIMEN_SIZE = 96;
export const DEFAULT_SPECIMEN_WEIGHT = 400;
export const DEFAULT_SORT_ORDER: LibrarySortOrder = 'name-asc';

export class LibrarySettings {
	search = $state('');
	origin = $state(NO_FILTERS.origin);
	format = $state(NO_FILTERS.format);
	technology = $state(NO_FILTERS.technology);
	spacing = $state(NO_FILTERS.spacing);
	status = $state(NO_FILTERS.status);
	sortOrder = $state<LibrarySortOrder>(DEFAULT_SORT_ORDER);
	specimenMode = $state<SpecimenMode>('names');
	specimenSize = $state(DEFAULT_SPECIMEN_SIZE);
	specimenWeight = $state(DEFAULT_SPECIMEN_WEIGHT);
	/** How much of the list is on screen. Grows only when somebody asks for more. */
	displayLimit = $state(PAGE_SIZE);

	/** The five narrowing choices, in the shape the filtering takes. */
	filters: LibraryFilters = $derived({
		origin: this.origin,
		format: this.format,
		technology: this.technology,
		spacing: this.spacing,
		status: this.status
	});

	/** Whether anything has been changed from the defaults, so Reset has something to undo. */
	isDirty = $derived(
		Boolean(this.search) ||
			this.filters.origin !== 'all' ||
			this.filters.format !== 'all' ||
			this.filters.technology !== 'all' ||
			this.filters.spacing !== 'all' ||
			this.filters.status !== 'all' ||
			this.sortOrder !== DEFAULT_SORT_ORDER ||
			this.specimenMode !== 'names' ||
			this.specimenSize !== DEFAULT_SPECIMEN_SIZE ||
			this.specimenWeight !== DEFAULT_SPECIMEN_WEIGHT
	);

	/**
	 * Narrowing or reordering the list puts it back to the first page.
	 *
	 * Keeping the old depth would leave somebody looking at the four hundredth match of a search
	 * they have just changed.
	 */
	private firstPage() {
		this.displayLimit = PAGE_SIZE;
	}

	setSearch(value: string) {
		this.search = value;
		this.firstPage();
	}

	setFilter(key: LibraryFilterKey, value: string) {
		if (key === 'origin') this.origin = value;
		if (key === 'format') this.format = value;
		if (key === 'technology') this.technology = value;
		if (key === 'spacing') this.spacing = value;
		if (key === 'status') this.status = value;
		if (key === 'sort') this.sortOrder = value as LibrarySortOrder;
		this.firstPage();
	}

	clearFilter(key: LibraryFilterKey) {
		this.setFilter(key, key === 'sort' ? DEFAULT_SORT_ORDER : 'all');
	}

	clearFilters() {
		this.origin = 'all';
		this.format = 'all';
		this.technology = 'all';
		this.spacing = 'all';
		this.status = 'all';
		this.firstPage();
	}

	/** Back to how the library looks on a first run, except for how far it is scrolled. */
	reset() {
		this.search = '';
		this.clearFilters();
		this.sortOrder = DEFAULT_SORT_ORDER;
		this.specimenMode = 'names';
		this.specimenSize = DEFAULT_SPECIMEN_SIZE;
		this.specimenWeight = DEFAULT_SPECIMEN_WEIGHT;
	}

	/** Shows more of the list, and says whether there was any more to show. */
	showMore(available: number): boolean {
		if (this.displayLimit >= available) return false;
		this.displayLimit += PAGE_SIZE;
		return true;
	}

	/**
	 * Takes back what a stored session held.
	 *
	 * The session has already been parsed and validated by `$lib/session`, which is where the
	 * rules about what a stored value may be belong; anything absent is simply left alone.
	 */
	apply(session: Partial<Session>) {
		if (session.search !== undefined) this.search = session.search;
		if (session.filters) {
			this.origin = session.filters.origin;
			this.format = session.filters.format;
			this.technology = session.filters.technology;
			this.spacing = session.filters.spacing;
			this.status = session.filters.status;
		}
		if (session.sortOrder) this.sortOrder = session.sortOrder;
		if (session.specimenMode) this.specimenMode = session.specimenMode;
		if (session.specimenSize !== undefined) this.specimenSize = session.specimenSize;
		if (session.specimenWeight !== undefined) this.specimenWeight = session.specimenWeight;
	}

	/**
	 * Shows at least this much of the list, never less.
	 *
	 * A session restore arrives after the first page is already drawn, and taking rows back off
	 * the screen would be a worse answer than ignoring the stored depth.
	 */
	showAtLeast(limit: number) {
		this.displayLimit = Math.max(this.displayLimit, limit);
	}
}
