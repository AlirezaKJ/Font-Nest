import { describe, expect, it, vi } from 'vitest';

import type { ImportCandidate } from '$lib/bindings/ImportCandidate';
import type { ImportOutcome } from '$lib/bindings/ImportOutcome';
import type { ImportPlan } from '$lib/bindings/ImportPlan';
import type { ImportVerdict } from '$lib/bindings/ImportVerdict';

import {
	embeddingLabel,
	importReviewed,
	importableCount,
	outcomeSummary,
	planSummary,
	reviewChosenFonts,
	verdictDetail,
	verdictLabel,
	verdictTone
} from './import';

const VERDICTS: ImportVerdict[] = [
	'installable',
	'alreadyInstalled',
	'repeatedSelection',
	'previewOnly',
	'unreadable',
	'tooLarge',
	'systemOwned',
	'missing'
];

function candidate(
	verdict: ImportVerdict,
	sourcePath = `C:/fonts/${verdict}.ttf`
): ImportCandidate {
	return {
		sourcePath,
		fileName: sourcePath.split('/').pop() ?? '',
		sizeBytes: 42_000,
		verdict,
		installedFileName: verdict === 'installable' ? 'FontNest-0123456789ab-One.ttf' : null,
		faces: [],
		licence: null
	};
}

function plan(candidates: ImportCandidate[]): ImportPlan {
	return { candidates, installableBytes: 0, truncated: false };
}

function outcome(overrides: Partial<ImportOutcome>): ImportOutcome {
	return {
		sourcePath: 'C:/fonts/One.ttf',
		fileName: 'One.ttf',
		installed: false,
		displayName: null,
		refusal: null,
		failure: null,
		...overrides
	};
}

describe('describing a verdict', () => {
	// Every verdict reaches the interface, so every verdict needs words for it. A missing case
	// would otherwise show up as an empty row in front of somebody deciding what to install.
	it('has words for every verdict the backend can return', () => {
		for (const verdict of VERDICTS) {
			expect(verdictLabel(verdict).length).toBeGreaterThan(0);
			expect(verdictDetail(verdict).length).toBeGreaterThan(0);
		}
	});

	it('marks only an installable file as ready', () => {
		expect(verdictTone('installable')).toBe('ready');
		expect(verdictTone('alreadyInstalled')).toBe('settled');
		expect(verdictTone('systemOwned')).toBe('blocked');
		expect(verdictTone('unreadable')).toBe('blocked');
	});

	it('states embedding permission in words, or says nothing', () => {
		const restricted = { ...candidate('installable') };
		restricted.licence = { description: null, url: null, embedding: 'restricted' };

		expect(embeddingLabel(restricted)).toBe('Embedding not permitted');
		expect(embeddingLabel(candidate('installable'))).toBeNull();
	});
});

describe('summarising a review', () => {
	it('counts what would actually be installed', () => {
		const reviewed = plan([
			candidate('installable', 'C:/fonts/One.ttf'),
			candidate('installable', 'C:/fonts/Two.otf'),
			candidate('alreadyInstalled')
		]);

		expect(importableCount(reviewed)).toBe(2);
		expect(planSummary(reviewed)).toBe('3 files, 2 ready to install.');
	});

	it('says plainly when nothing can be installed', () => {
		expect(planSummary(plan([candidate('previewOnly')]))).toBe(
			'1 file, none of which can be installed.'
		);
		expect(planSummary(plan([]))).toBe('Nothing to import.');
	});

	it('does not qualify a selection that is entirely ready', () => {
		expect(planSummary(plan([candidate('installable')]))).toBe('1 file, all ready to install.');
	});
});

describe('summarising what happened', () => {
	it('reports installs and failures separately', () => {
		const outcomes = [
			outcome({ installed: true, displayName: 'One Regular' }),
			outcome({ failure: 'the registry refused the font' })
		];

		expect(outcomeSummary(outcomes)).toBe('1 font installed, 1 file could not be.');
	});

	// A file refused for a reason the review already gave is not a failure, it is the plan.
	it('does not count a reviewed refusal as a failure', () => {
		const outcomes = [
			outcome({ installed: true, displayName: 'One Regular' }),
			outcome({ refusal: 'alreadyInstalled' })
		];

		expect(outcomeSummary(outcomes)).toBe('1 font installed.');
	});

	it('says when nothing was installed', () => {
		expect(outcomeSummary([outcome({ refusal: 'systemOwned' })])).toBe(
			'Nothing was installed.'
		);
	});
});

describe('running an import', () => {
	it('sends only the files the review found installable', async () => {
		const run = vi.fn().mockResolvedValue([]);
		const reviewed = plan([
			candidate('installable', 'C:/fonts/One.ttf'),
			candidate('previewOnly', 'C:/fonts/Collection.ttc'),
			candidate('systemOwned', 'C:/Windows/Fonts/arial.ttf')
		]);

		await importReviewed(reviewed, run);

		expect(run).toHaveBeenCalledWith(['C:/fonts/One.ttf']);
	});

	it('asks for nothing when there is nothing to install', async () => {
		const run = vi.fn().mockResolvedValue([]);

		await expect(importReviewed(plan([candidate('unreadable')]), run)).resolves.toEqual([]);
		expect(run).not.toHaveBeenCalled();
	});

	// Dismissing the picker is not the same as choosing files that cannot be installed, and the
	// interface needs to tell them apart to know whether to open a review at all.
	it('tells a dismissed picker apart from an empty result', async () => {
		const preflight = vi.fn().mockResolvedValue(plan([]));

		const dismissed = await reviewChosenFonts(false, async () => [], preflight);

		expect(dismissed).toBeNull();
		expect(preflight).not.toHaveBeenCalled();
	});

	it('reviews what the picker returned', async () => {
		const reviewed = plan([candidate('installable')]);
		const preflight = vi.fn().mockResolvedValue(reviewed);

		const result = await reviewChosenFonts(false, async () => ['C:/fonts/One.ttf'], preflight);

		expect(result).toBe(reviewed);
		expect(preflight).toHaveBeenCalledWith(['C:/fonts/One.ttf']);
	});
});
