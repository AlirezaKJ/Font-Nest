import { describe, expect, it } from 'vitest';

import {
	parseSession,
	reconcileSession,
	restorableSortOrder,
	restorableView,
	type Session
} from './session';

const FAMILY = 'family:0123456789abcdef0123456789abcdef';
const OTHER_FAMILY = 'family:fedcba9876543210fedcba9876543210';

function storedSession(overrides: Record<string, unknown> = {}): Record<string, unknown> {
	return {
		view: 'preview',
		selectedFamilyId: FAMILY,
		search: 'grotesk',
		filters: {
			origin: 'current-user',
			format: 'TrueType',
			technology: 'variable',
			spacing: 'all',
			status: 'all'
		},
		sortOrder: 'name-desc',
		specimenMode: 'custom',
		specimenSize: 120,
		specimenWeight: 650,
		previewSize: 72,
		previewWeight: 500,
		displayLimit: 360,
		scrollTop: 4200,
		...overrides
	};
}

describe('parseSession', () => {
	it('reads back a session it wrote', () => {
		expect(parseSession(storedSession())).toEqual({
			view: 'preview',
			selectedFamilyId: FAMILY,
			search: 'grotesk',
			filters: {
				origin: 'current-user',
				format: 'TrueType',
				technology: 'variable',
				spacing: 'all',
				status: 'all'
			},
			sortOrder: 'name-desc',
			specimenMode: 'custom',
			specimenSize: 120,
			specimenWeight: 650,
			previewSize: 72,
			previewWeight: 500,
			displayLimit: 360,
			scrollTop: 4200
		} satisfies Partial<Session>);
	});

	it('keeps nothing from a session that was never stored', () => {
		expect(parseSession(undefined)).toEqual({});
		expect(parseSession(null)).toEqual({});
		expect(parseSession('library')).toEqual({});
	});

	// Storage is a file a person can edit, and an older build may have written something this one
	// has never heard of. A bad field is dropped; the rest of the session still opens.
	it('drops a single bad field without losing the rest', () => {
		const session = parseSession(storedSession({ view: 'nowhere', specimenSize: 'large' }));

		expect(session.view).toBeUndefined();
		expect(session.specimenSize).toBeUndefined();
		expect(session.search).toBe('grotesk');
		expect(session.sortOrder).toBe('name-desc');
	});

	it('refuses a family identifier that is not one', () => {
		expect(
			parseSession(storedSession({ selectedFamilyId: 'family:../../etc' })).selectedFamilyId
		).toBeUndefined();
		expect(
			parseSession(storedSession({ selectedFamilyId: 'face:0123' })).selectedFamilyId
		).toBeUndefined();
	});

	it('refuses sizes and positions outside any sensible range', () => {
		const session = parseSession(
			storedSession({
				specimenSize: 10_000,
				previewWeight: 0,
				scrollTop: -1,
				displayLimit: Number.POSITIVE_INFINITY
			})
		);

		expect(session.specimenSize).toBeUndefined();
		expect(session.previewWeight).toBeUndefined();
		expect(session.scrollTop).toBeUndefined();
		expect(session.displayLimit).toBeUndefined();
	});

	it('fills in the filters a partial record left out', () => {
		const session = parseSession(storedSession({ filters: { origin: 'all-users' } }));

		expect(session.filters).toEqual({
			origin: 'all-users',
			format: 'all',
			technology: 'all',
			spacing: 'all',
			status: 'all'
		});
	});

	it('refuses a search long enough to be a paste accident', () => {
		expect(parseSession(storedSession({ search: 'a'.repeat(201) })).search).toBeUndefined();
		expect(parseSession(storedSession({ search: 'a'.repeat(200) })).search).toHaveLength(200);
	});
});

describe('reconcileSession', () => {
	// The font you had open can be uninstalled between one launch and the next.
	it('drops a family that is no longer installed', () => {
		const session = reconcileSession(
			{ view: 'preview', selectedFamilyId: FAMILY },
			(familyId) => familyId === OTHER_FAMILY
		);

		expect(session.selectedFamilyId).toBeNull();
		expect(session.view).toBe('library');
	});

	it('keeps a family that is still there', () => {
		const session = reconcileSession(
			{ view: 'preview', selectedFamilyId: FAMILY },
			(familyId) => familyId === FAMILY
		);

		expect(session.selectedFamilyId).toBe(FAMILY);
		expect(session.view).toBe('preview');
	});

	it('leaves a view that does not depend on a family alone', () => {
		const session = reconcileSession(
			{ view: 'discover', selectedFamilyId: FAMILY },
			() => false
		);

		expect(session.view).toBe('discover');
		expect(session.selectedFamilyId).toBeNull();
	});
});

describe('what gets written down', () => {
	it('sends a What is New session back to the library', () => {
		expect(restorableView('whatsNew')).toBe('library');
		expect(restorableView('discover')).toBe('discover');
	});

	it('falls back to the default sort when the stored one is unknown', () => {
		expect(restorableSortOrder('faces')).toBe('faces');
		expect(restorableSortOrder('by-vibes')).toBe('name-asc');
	});
});
