import { describe, expect, it } from 'vitest';

import {
	DEFAULT_SPECIMEN_SIZE,
	DEFAULT_SPECIMEN_WEIGHT,
	LibrarySettings,
	PAGE_SIZE
} from './settings.svelte';

describe('the library settings', () => {
	it('starts with nothing searched, nothing filtered and one page shown', () => {
		const settings = new LibrarySettings();

		expect(settings.search).toBe('');
		expect(settings.filters).toEqual({
			origin: 'all',
			format: 'all',
			technology: 'all',
			spacing: 'all',
			status: 'all'
		});
		expect(settings.displayLimit).toBe(PAGE_SIZE);
		expect(settings.isDirty).toBe(false);
	});

	it('knows when something has been changed from the defaults', () => {
		const settings = new LibrarySettings();
		settings.setSearch('inter');
		expect(settings.isDirty).toBe(true);

		const filtered = new LibrarySettings();
		filtered.setFilter('format', 'WOFF2');
		expect(filtered.isDirty).toBe(true);

		const resized = new LibrarySettings();
		resized.specimenSize = DEFAULT_SPECIMEN_SIZE + 20;
		expect(resized.isDirty).toBe(true);
	});

	// Keeping the depth would leave somebody looking at the four hundredth match of a search they
	// have just changed.
	it('goes back to the first page whenever the list is narrowed or reordered', () => {
		const settings = new LibrarySettings();
		settings.showMore(10_000);
		expect(settings.displayLimit).toBe(PAGE_SIZE * 2);

		settings.setSearch('mono');
		expect(settings.displayLimit).toBe(PAGE_SIZE);

		settings.showMore(10_000);
		settings.setFilter('sort', 'name-desc');
		expect(settings.displayLimit).toBe(PAGE_SIZE);

		settings.showMore(10_000);
		settings.clearFilters();
		expect(settings.displayLimit).toBe(PAGE_SIZE);
	});

	it('will not show more than there is', () => {
		const settings = new LibrarySettings();
		expect(settings.showMore(PAGE_SIZE)).toBe(false);
		expect(settings.displayLimit).toBe(PAGE_SIZE);
		expect(settings.showMore(PAGE_SIZE + 1)).toBe(true);
	});

	it('clears one filter back to its own default, and sort back to by name', () => {
		const settings = new LibrarySettings();
		settings.setFilter('origin', 'userInstalled');
		settings.setFilter('sort', 'styles');

		settings.clearFilter('origin');
		expect(settings.filters.origin).toBe('all');

		settings.clearFilter('sort');
		expect(settings.sortOrder).toBe('name-asc');
	});

	it('clears the filters without touching the search or the specimen', () => {
		const settings = new LibrarySettings();
		settings.setSearch('cardo');
		settings.specimenSize = 140;
		settings.setFilter('status', 'conflict');

		settings.clearFilters();

		expect(settings.filters.status).toBe('all');
		expect(settings.search).toBe('cardo');
		expect(settings.specimenSize).toBe(140);
	});

	it('puts everything back on a reset', () => {
		const settings = new LibrarySettings();
		settings.setSearch('cardo');
		settings.setFilter('origin', 'userInstalled');
		settings.setFilter('sort', 'faces');
		settings.specimenMode = 'custom';
		settings.specimenSize = 140;
		settings.specimenWeight = 700;

		settings.reset();

		expect(settings.isDirty).toBe(false);
		expect(settings.specimenSize).toBe(DEFAULT_SPECIMEN_SIZE);
		expect(settings.specimenWeight).toBe(DEFAULT_SPECIMEN_WEIGHT);
		expect(settings.specimenMode).toBe('names');
	});
});

describe('carrying the settings across a restart', () => {
	it('takes back what the stored session held', () => {
		const settings = new LibrarySettings();
		settings.apply({
			search: 'mono',
			filters: {
				origin: 'userInstalled',
				format: 'all',
				technology: 'variable',
				spacing: 'all',
				status: 'all'
			},
			sortOrder: 'styles',
			specimenMode: 'custom',
			specimenSize: 140,
			specimenWeight: 700
		});

		expect(settings.search).toBe('mono');
		expect(settings.filters.origin).toBe('userInstalled');
		expect(settings.filters.technology).toBe('variable');
		expect(settings.sortOrder).toBe('styles');
		expect(settings.specimenMode).toBe('custom');
		expect(settings.specimenSize).toBe(140);
		expect(settings.specimenWeight).toBe(700);
	});

	// A session written by an older version can be missing anything, and what it leaves out should
	// stay at its default rather than being blanked.
	it('leaves out what the session does not mention', () => {
		const settings = new LibrarySettings();
		settings.specimenSize = 140;
		settings.apply({ search: 'cardo' });

		expect(settings.search).toBe('cardo');
		expect(settings.specimenSize).toBe(140);
		expect(settings.filters.origin).toBe('all');
	});

	it('takes nothing at all from an empty session', () => {
		const settings = new LibrarySettings();
		settings.apply({});
		expect(settings.isDirty).toBe(false);
	});

	// The restore arrives after the first page is already drawn, so a smaller stored depth must
	// not take rows back off the screen.
	it('never shows less than is already on screen', () => {
		const settings = new LibrarySettings();
		settings.showMore(10_000);
		settings.showAtLeast(PAGE_SIZE);
		expect(settings.displayLimit).toBe(PAGE_SIZE * 2);

		settings.showAtLeast(PAGE_SIZE * 5);
		expect(settings.displayLimit).toBe(PAGE_SIZE * 5);
	});
});
