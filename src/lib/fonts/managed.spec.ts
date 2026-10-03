import { describe, expect, it } from 'vitest';

import type { FontRemovalRefusal } from '$lib/bindings/FontRemovalRefusal';
import type { ManagedFontSummary } from '$lib/bindings/ManagedFontSummary';
import type { QuarantinedFontSummary } from '$lib/bindings/QuarantinedFontSummary';

import {
	EMPTY_INVENTORY,
	byFamily,
	formatBytes,
	inventorySummary,
	refusalDetail,
	removedAgo,
	setAsideBytes,
	sourceLabel
} from './managed';

const REFUSALS: FontRemovalRefusal[] = [
	'unknownSource',
	'locationUnavailable',
	'recordMismatch',
	'missing',
	'redirected',
	'outsideFontFolder',
	'changed',
	'notRegistered',
	'protected',
	'unreadable'
];

function installed(
	displayName: string,
	familyName = 'Inter',
	imported = false
): ManagedFontSummary {
	return {
		id: `${imported ? 'local' : 'google-fonts'}:${displayName}`,
		provider: imported ? 'local' : 'google-fonts',
		familyName,
		displayName,
		imported
	};
}

function setAside(
	displayName: string,
	removedAt: number,
	sizeBytes = 42_000
): QuarantinedFontSummary {
	return {
		id: `google-fonts:${displayName}`,
		provider: 'google-fonts',
		familyName: 'Inter',
		displayName,
		imported: false,
		removedAt,
		sizeBytes
	};
}

describe('saying where a font came from', () => {
	it('tells a provider font apart from one that was imported', () => {
		expect(sourceLabel(installed('Inter Regular'))).toBe('Discover');
		expect(sourceLabel(installed('Cardo Regular', 'Cardo', true))).toBe('Imported');
	});
});

describe('explaining a refusal', () => {
	// A refusal is the interface telling somebody why their font is still there. Every reason the
	// backend can give needs words, or a refusal shows up as a blank explanation.
	it('has words for every reason a removal can refuse', () => {
		for (const reason of REFUSALS) {
			const detail = refusalDetail(reason);
			expect(detail.length).toBeGreaterThan(0);
			expect(detail).not.toBe(reason);
		}
	});

	it('distinguishes a replaced font from an edited record', () => {
		expect(refusalDetail('changed')).not.toBe(refusalDetail('recordMismatch'));
	});
});

describe('summarising the inventory', () => {
	it('says plainly when FontNest has installed nothing', () => {
		expect(inventorySummary(EMPTY_INVENTORY)).toBe(
			'FontNest has not installed any fonts on this computer.'
		);
	});

	it('counts what is installed', () => {
		expect(inventorySummary({ installed: [installed('Inter Regular')], quarantined: [] })).toBe(
			'1 font installed by FontNest.'
		);
	});

	// Fonts that were removed are still on the computer, set aside. Leaving them out of the
	// summary would make them look gone.
	it('mentions what has been set aside', () => {
		const summary = inventorySummary({
			installed: [installed('Inter Regular'), installed('Inter Bold')],
			quarantined: [setAside('Inter Thin', 1_790_000_000)]
		});

		expect(summary).toBe('2 fonts installed by FontNest, and 1 more set aside.');
	});
});

describe('grouping by family', () => {
	it('keeps a family with several styles together', () => {
		const grouped = byFamily([
			installed('Inter Regular'),
			installed('Cardo Regular', 'Cardo'),
			installed('Inter Bold')
		]);

		expect(grouped.map(([family]) => family)).toEqual(['Inter', 'Cardo']);
		expect(grouped[0][1]).toHaveLength(2);
	});

	it('reads an empty inventory as no families', () => {
		expect(byFamily([])).toEqual([]);
	});
});

describe('how long ago a font was removed', () => {
	const now = 1_790_000_000_000;
	const seconds = now / 1000;

	it('speaks in units a person would use', () => {
		expect(removedAgo(seconds, now)).toBe('just now');
		expect(removedAgo(seconds - 600, now)).toBe('10 minutes ago');
		expect(removedAgo(seconds - 3600, now)).toBe('an hour ago');
		expect(removedAgo(seconds - 7200, now)).toBe('2 hours ago');
		expect(removedAgo(seconds - 86_400, now)).toBe('yesterday');
		expect(removedAgo(seconds - 3 * 86_400, now)).toBe('3 days ago');
	});

	// A clock that disagrees with the record should not produce "in -4 minutes".
	it('never counts forwards', () => {
		expect(removedAgo(seconds + 600, now)).toBe('just now');
	});
});

describe('naming the space set-aside fonts hold', () => {
	it('reads sizes in units a person uses', () => {
		expect(formatBytes(0)).toBe('0 KB');
		expect(formatBytes(400)).toBe('1 KB');
		expect(formatBytes(42_000)).toBe('41 KB');
		expect(formatBytes(3_500_000)).toBe('3.3 MB');
	});

	// The space is the only reason to delete something that can still be put back, so the figure
	// is what the decision rests on.
	it('adds up everything waiting to be reclaimed', () => {
		expect(
			setAsideBytes([setAside('Inter Thin', 1, 1000), setAside('Inter Bold', 2, 2400)])
		).toBe(3400);
		expect(setAsideBytes([])).toBe(0);
	});
});
