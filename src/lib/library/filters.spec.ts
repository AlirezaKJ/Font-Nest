import { describe, expect, it } from 'vitest';

import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';

import {
	NO_FILTERS,
	activeFiltersFrom,
	familyHaystack,
	filterFamilies,
	formatValues,
	matchesFilters,
	matchesSearch,
	optionLabel,
	searchTerms,
	sortFamilies
} from './filters';

function family(overrides: Partial<FontFamilySummary> = {}): FontFamilySummary {
	return {
		id: `family:${overrides.name ?? 'Inter'}`,
		name: 'Inter',
		faceCount: 4,
		fileCount: 4,
		styles: ['Regular', 'Bold'],
		weights: [400, 700],
		formats: ['TrueType'],
		origins: ['userInstalled'],
		monospaced: false,
		variable: false,
		hasConflict: false,
		faces: [],
		...overrides
	};
}

describe('reading a search', () => {
	it('takes each word as its own requirement', () => {
		expect(searchTerms('  mono   variable ')).toEqual(['mono', 'variable']);
		expect(searchTerms('')).toEqual([]);
		expect(searchTerms('   ')).toEqual([]);
	});

	// Both words have to be found. Either-or would make a second word widen the results, which is
	// the opposite of what typing more is for.
	it('needs every word, not any of them', () => {
		const mono = family({ name: 'Roboto Mono', monospaced: true });
		expect(matchesSearch(mono, searchTerms('roboto mono'))).toBe(true);
		expect(matchesSearch(mono, searchTerms('roboto inter'))).toBe(false);
	});

	it('matches a family with no search at all', () => {
		expect(matchesSearch(family(), [])).toBe(true);
	});

	it('ignores the case of both the search and the family', () => {
		expect(matchesSearch(family({ name: 'Inter' }), searchTerms('INTER'))).toBe(true);
	});

	// Searching for what is on screen should find it, and the screen shows the origin's label.
	it('searches the words a person can actually see', () => {
		const haystack = familyHaystack(
			family({ name: 'Cardo', styles: ['Italic'], formats: ['OpenType'], variable: true })
		);
		expect(haystack).toContain('cardo');
		expect(haystack).toContain('italic');
		expect(haystack).toContain('opentype');
		expect(haystack).toContain('variable');
	});

	it('describes a static family as static, so that word finds it', () => {
		expect(familyHaystack(family({ variable: false }))).toContain('static');
	});
});

describe('narrowing by the filters', () => {
	it('keeps everything while the filters are untouched', () => {
		expect(matchesFilters(family(), NO_FILTERS)).toBe(true);
	});

	it('narrows by origin, format, technology and spacing', () => {
		const target = family({
			origins: ['machineInstalled'],
			formats: ['WOFF2'],
			variable: true,
			monospaced: true
		});
		expect(matchesFilters(target, { ...NO_FILTERS, origin: 'machineInstalled' })).toBe(true);
		expect(matchesFilters(target, { ...NO_FILTERS, origin: 'userInstalled' })).toBe(false);
		expect(matchesFilters(target, { ...NO_FILTERS, format: 'WOFF2' })).toBe(true);
		expect(matchesFilters(target, { ...NO_FILTERS, format: 'TrueType' })).toBe(false);
		expect(matchesFilters(target, { ...NO_FILTERS, technology: 'variable' })).toBe(true);
		expect(matchesFilters(target, { ...NO_FILTERS, technology: 'static' })).toBe(false);
		expect(matchesFilters(target, { ...NO_FILTERS, spacing: 'monospaced' })).toBe(true);
		expect(matchesFilters(target, { ...NO_FILTERS, spacing: 'proportional' })).toBe(false);
	});

	// A family can carry several origins, and any one of them should satisfy the filter.
	it('matches a family on any origin it has', () => {
		const both = family({ origins: ['machineInstalled', 'userInstalled'] });
		expect(matchesFilters(both, { ...NO_FILTERS, origin: 'userInstalled' })).toBe(true);
	});

	// Status is the only filter that is a flag rather than a choice: it means "only the ones
	// needing attention", so anything other than `all` means conflicts.
	it('treats status as showing only families with a conflict', () => {
		expect(
			matchesFilters(family({ hasConflict: true }), { ...NO_FILTERS, status: 'conflict' })
		).toBe(true);
		expect(
			matchesFilters(family({ hasConflict: false }), { ...NO_FILTERS, status: 'conflict' })
		).toBe(false);
	});
});

describe('ordering families', () => {
	const names = (families: FontFamilySummary[]) => families.map((entry) => entry.name);

	it('orders by name both ways', () => {
		const given = [family({ name: 'Cardo' }), family({ name: 'Inter' })];
		expect(names(sortFamilies(given, 'name-asc'))).toEqual(['Cardo', 'Inter']);
		expect(names(sortFamilies(given, 'name-desc'))).toEqual(['Inter', 'Cardo']);
	});

	it('orders by style count and by file count, heaviest first', () => {
		const given = [
			family({ name: 'Two', faceCount: 2, fileCount: 9 }),
			family({ name: 'Nine', faceCount: 9, fileCount: 2 })
		];
		expect(names(sortFamilies(given, 'styles'))).toEqual(['Nine', 'Two']);
		expect(names(sortFamilies(given, 'faces'))).toEqual(['Two', 'Nine']);
	});

	// Without the fallback, families that tie would come out in whatever order the scan reached
	// them, so the same catalogue would list differently from one run to the next.
	it('settles a tie by name', () => {
		const given = [
			family({ name: 'Inter', faceCount: 4 }),
			family({ name: 'Cardo', faceCount: 4 })
		];
		expect(names(sortFamilies(given, 'styles'))).toEqual(['Cardo', 'Inter']);
	});

	it('leaves the array it was handed in its original order', () => {
		const given = [family({ name: 'Inter' }), family({ name: 'Cardo' })];
		sortFamilies(given, 'name-asc');
		expect(names(given)).toEqual(['Inter', 'Cardo']);
	});
});

describe('the list the library shows', () => {
	const catalogue = [
		family({ name: 'Inter', origins: ['machineInstalled'], faceCount: 8 }),
		family({ name: 'Cardo', origins: ['userInstalled'], faceCount: 2, hasConflict: true }),
		family({ name: 'Roboto Mono', monospaced: true, variable: true, faceCount: 4 })
	];

	it('applies the search and the filters together', () => {
		const shown = filterFamilies(
			catalogue,
			'o',
			{ ...NO_FILTERS, spacing: 'monospaced' },
			'name-asc'
		);
		expect(shown.map((entry) => entry.name)).toEqual(['Roboto Mono']);
	});

	it('returns everything in order when nothing is asked of it', () => {
		expect(
			filterFamilies(catalogue, '', NO_FILTERS, 'name-asc').map((entry) => entry.name)
		).toEqual(['Cardo', 'Inter', 'Roboto Mono']);
	});

	it('comes back empty rather than unfiltered when nothing matches', () => {
		expect(filterFamilies(catalogue, 'nothing-by-this-name', NO_FILTERS, 'name-asc')).toEqual(
			[]
		);
	});

	it('offers each format present once, in order', () => {
		expect(
			formatValues([
				family({ formats: ['TrueType', 'WOFF2'] }),
				family({ formats: ['TrueType'] })
			])
		).toEqual(['TrueType', 'WOFF2']);
	});
});

describe('naming the filters that are narrowing the list', () => {
	const groups = [
		{
			key: 'origin',
			value: 'userInstalled',
			options: [
				{ value: 'all', label: 'Anywhere' },
				{ value: 'userInstalled', label: 'Installed' }
			]
		},
		{
			key: 'format',
			value: 'all',
			options: [
				{ value: 'all', label: 'All formats' },
				{ value: 'WOFF2', label: 'WOFF2' }
			]
		}
	];

	it('names only the filters that are actually set', () => {
		expect(activeFiltersFrom(groups)).toEqual([{ key: 'origin', label: 'Installed' }]);
	});

	it('comes back empty when nothing is filtered', () => {
		expect(activeFiltersFrom(groups.map((group) => ({ ...group, value: 'all' })))).toEqual([]);
	});

	// The chip is the only place the choice is written out, so it has to read as the control does.
	it('uses the label the control shows, not the value underneath', () => {
		expect(optionLabel(groups[0].options, 'userInstalled')).toBe('Installed');
	});

	it('falls back to the value when a control has no label for it', () => {
		expect(optionLabel(groups[0].options, 'machineInstalled')).toBe('machineInstalled');
	});
});
