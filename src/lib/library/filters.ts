/**
 * Choosing which families the library shows, and in what order.
 *
 * This is the behaviour the catalogue is judged on and it lived inside the route as derived
 * state, where it could not be tested: searching across five fields at once, five filters that
 * each narrow it further, and four orders. Here it is a function of its arguments, so each rule
 * can be stated once and checked.
 */

import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
import type { FontOrigin } from '$lib/bindings/FontOrigin';
import { fontOrigin } from '$lib/fonts/font-origin';

export type LibrarySortOrder = 'name-asc' | 'name-desc' | 'styles' | 'faces';

/** Which control a filter chip stands for, so dismissing the chip clears the right one. */
export type LibraryFilterKey = 'origin' | 'format' | 'technology' | 'spacing' | 'status' | 'sort';

/** A filter currently narrowing the list, named the way its chip shows it. */
export type ActiveLibraryFilter = { key: LibraryFilterKey; label: string };

/** The five narrowing choices, each `all` until somebody picks something. */
export type LibraryFilters = {
	origin: string;
	format: string;
	technology: string;
	spacing: string;
	status: string;
};

export const NO_FILTERS: LibraryFilters = {
	origin: 'all',
	format: 'all',
	technology: 'all',
	spacing: 'all',
	status: 'all'
};

/**
 * The words a search is made of.
 *
 * Every word has to be found, so "mono var" means both rather than either.
 */
export function searchTerms(search: string): string[] {
	return search
		.toLocaleLowerCase()
		.split(/\s+/)
		.map((term) => term.trim())
		.filter(Boolean);
}

/**
 * Everything about a family a search can match, as one lowercase string.
 *
 * Origins are matched by the label a person reads rather than the identifier underneath, so
 * searching the word that is on screen finds the family that shows it.
 */
export function familyHaystack(family: FontFamilySummary): string {
	return [
		family.name,
		...family.styles,
		...family.origins.map((origin) => fontOrigin(origin).label),
		...family.formats,
		family.variable ? 'Variable' : 'Static'
	]
		.join(' ')
		.toLocaleLowerCase();
}

export function matchesSearch(family: FontFamilySummary, terms: string[]): boolean {
	const searchable = familyHaystack(family);
	return terms.every((term) => searchable.includes(term));
}

export function matchesFilters(family: FontFamilySummary, filters: LibraryFilters): boolean {
	return (
		(filters.origin === 'all' || family.origins.includes(filters.origin as FontOrigin)) &&
		(filters.format === 'all' || family.formats.includes(filters.format)) &&
		(filters.technology === 'all' || family.variable === (filters.technology === 'variable')) &&
		(filters.spacing === 'all' || family.monospaced === (filters.spacing === 'monospaced')) &&
		(filters.status === 'all' || family.hasConflict)
	);
}

/**
 * Orders families, leaving the array it was given alone.
 *
 * Every order falls back to the name, so families that tie on style or file count still come out
 * in a stable, readable order rather than whichever the scan happened to reach first.
 */
export function sortFamilies(
	families: FontFamilySummary[],
	order: LibrarySortOrder
): FontFamilySummary[] {
	return [...families].sort((left, right) => {
		if (order === 'name-desc') return right.name.localeCompare(left.name);
		if (order === 'styles')
			return right.faceCount - left.faceCount || left.name.localeCompare(right.name);
		if (order === 'faces')
			return right.fileCount - left.fileCount || left.name.localeCompare(right.name);
		return left.name.localeCompare(right.name);
	});
}

/** What the library shows: everything matching the search and the filters, in order. */
export function filterFamilies(
	families: FontFamilySummary[],
	search: string,
	filters: LibraryFilters,
	order: LibrarySortOrder
): FontFamilySummary[] {
	const terms = searchTerms(search);
	return sortFamilies(
		families.filter(
			(family) => matchesSearch(family, terms) && matchesFilters(family, filters)
		),
		order
	);
}

/** The formats present in a catalogue, for the filter to offer. */
export function formatValues(families: FontFamilySummary[]): string[] {
	return [...new Set(families.flatMap((family) => family.formats))].sort();
}

/** Enough of a filter control for naming what it has been set to. */
type FilterGroupLike = {
	key: string;
	value: string;
	options: readonly { value: string; label: string }[];
};

/** The label a control is showing for its current value, or the raw value if it has none. */
export function optionLabel(
	options: readonly { value: string; label: string }[],
	value: string
): string {
	return options.find((option) => option.value === value)?.label ?? value;
}

/**
 * The filters currently narrowing the list, as the chips name them.
 *
 * The chips are the only place active choices are written out, since the control that sets them no
 * longer shows its value. Sort is not among them: it always has a value, so a chip for it could
 * never be dismissed.
 */
export function activeFiltersFrom(groups: readonly FilterGroupLike[]): ActiveLibraryFilter[] {
	return groups
		.filter((group) => group.value !== 'all')
		.map((group) => ({
			key: group.key as LibraryFilterKey,
			label: optionLabel(group.options, group.value)
		}));
}
