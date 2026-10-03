import { render, screen } from '@testing-library/svelte';
import type { ComponentProps } from 'svelte';
import { describe, expect, it, vi } from 'vitest';

import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
import { NO_FILTERS, filterFamilies } from '$lib/library/filters';

import LibraryView from './LibraryView.svelte';

/**
 * The library is the view FontNest opens on, and it was 400 lines of markup inside the route with
 * nothing covering it. These check what it shows for a given catalogue, and that each of its empty
 * states says something a person can act on rather than nothing at all.
 */

function family(name: string, overrides: Partial<FontFamilySummary> = {}): FontFamilySummary {
	return {
		id: `family:${name}`,
		name,
		faceCount: 2,
		fileCount: 2,
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

function catalogue(families: FontFamilySummary[]): FontCatalogue {
	return {
		families,
		familyCount: families.length,
		faceCount: families.reduce((total, entry) => total + entry.faceCount, 0),
		conflictCount: 0,
		scanDurationMs: 42
	};
}

const FAMILIES = [family('Cardo'), family('Inter'), family('Roboto Mono', { monospaced: true })];

function mount(overrides: Partial<ComponentProps<typeof LibraryView>> = {}) {
	const spies = {
		onClearFilter: vi.fn(),
		onClearFilters: vi.fn(),
		onResetAll: vi.fn(),
		onUpdateFilter: vi.fn(),
		onUpdateSearch: vi.fn(),
		onSetPreviewText: vi.fn(),
		onRefreshCatalogue: vi.fn(),
		onImport: vi.fn(),
		onPreviewFile: vi.fn(),
		onOpenFamilyPreview: vi.fn(),
		onToggleFamily: vi.fn(),
		onTogglePinned: vi.fn(),
		onReviewConflict: vi.fn()
	};
	const result = render(LibraryView, {
		catalogue: catalogue(FAMILIES),
		catalogueMode: 'native',
		loading: false,
		errorMessage: null,
		prefersReducedMotion: true,
		density: 'comfortable',
		pageSize: 120,
		previewText: '',
		pinnedFamilyIds: [],
		selectedFamilyId: null,
		filterGroups: [],
		activeFilters: [],
		filteredFamilies: filterFamilies(FAMILIES, '', NO_FILTERS, 'name-asc'),
		hasResettableState: false,
		sortOrder: 'name-asc',
		sortOptions: [{ value: 'name-asc', label: 'Name A–Z' }],
		search: '',
		specimenMode: 'names',
		specimenSize: 96,
		specimenWeight: 400,
		displayLimit: 120,
		libraryScrollElement: undefined,
		familyMenu: () => ({ entries: [] }),
		faceMenu: () => ({ entries: [] }),
		...spies,
		...overrides
	});
	return { ...spies, container: result.container };
}

describe('the library view', () => {
	it('lists every family it was given, in the order it was given them', () => {
		const { container } = mount();

		const names = [...container.querySelectorAll('.specimen-toggle .family-line strong')].map(
			(name) => name.textContent
		);
		expect(names).toEqual(['Cardo', 'Inter', 'Roboto Mono']);
	});

	it('says how much is installed, and how much of it is being shown', () => {
		const { container } = mount();

		expect(screen.getByRole('heading', { name: 'Your fonts' })).toBeDefined();
		expect(container.querySelector('.catalogue-summary')?.textContent).toContain('3 families');
	});

	// Showing nothing with no explanation is the one outcome none of these states may produce.
	it('explains an empty catalogue and offers a way on', () => {
		const spies = mount({ catalogue: catalogue([]), filteredFamilies: [] });

		expect(screen.getByRole('heading', { name: 'No installed fonts found' })).toBeDefined();
		screen.getByRole('button', { name: 'Preview a font file' }).click();
		expect(spies.onPreviewFile).toHaveBeenCalledOnce();
	});

	it('tells a failed scan apart from an empty one, and offers to scan again', () => {
		const spies = mount({ errorMessage: 'the font folder could not be read' });

		expect(screen.getByRole('alert').textContent).toContain(
			'the font folder could not be read'
		);
		screen.getByRole('button', { name: 'Scan again' }).click();
		expect(spies.onRefreshCatalogue).toHaveBeenCalledOnce();
	});

	// A search that matches nothing is not the same as owning no fonts.
	it('says when a search has hidden everything, rather than looking empty', () => {
		mount({ search: 'nothing-matches-this', filteredFamilies: [] });

		expect(screen.getByRole('heading', { name: 'No families match' })).toBeDefined();
	});

	it('shows only as much of a long list as the display limit allows', () => {
		const many = Array.from({ length: 30 }, (_, index) =>
			family(`Family ${String(index).padStart(2, '0')}`)
		);
		mount({
			catalogue: catalogue(many),
			filteredFamilies: many,
			displayLimit: 10
		});

		expect(screen.getAllByRole('button', { name: /^Family \d\d/ })).toHaveLength(10);
		// The row that offers the rest has to say how many are left, or the list looks truncated.
		expect(screen.getByText(/of 30/)).toBeDefined();
	});

	it('asks the route to open a family rather than deciding by itself', () => {
		const spies = mount();

		screen.getAllByRole('button', { name: /Cardo/ })[0].click();
		expect(spies.onToggleFamily).toHaveBeenCalledWith('family:Cardo');
	});
});
