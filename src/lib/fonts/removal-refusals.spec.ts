import { describe, expect, it } from 'vitest';

import type { RefusedFontRemoval } from '$lib/bindings/RefusedFontRemoval';
import { removalRefusalNote } from './removal-refusals';

function refused(
	artifactId: string,
	reason: RefusedFontRemoval['reason'] = 'changed'
): RefusedFontRemoval {
	return { artifactId, displayName: 'Inter Regular', reason };
}

describe('removalRefusalNote', () => {
	it('says nothing when every font was removed', () => {
		expect(removalRefusalNote([])).toBe('');
	});

	it('names the check that stopped a single font', () => {
		expect(removalRefusalNote([refused('gf:inter:regular')])).toBe(
			'FontNest kept 1 file because the file has changed since FontNest installed it.'
		);
	});

	it('groups fonts that were kept for the same reason', () => {
		const note = removalRefusalNote([
			refused('gf:inter:regular'),
			refused('gf:inter:bold'),
			refused('gf:inter:italic')
		]);

		expect(note).toBe(
			'FontNest kept 3 files because the file has changed since FontNest installed it.'
		);
	});

	it('lists every reason when fonts were kept for different ones', () => {
		const note = removalRefusalNote([
			refused('gf:inter:regular', 'missing'),
			refused('gf:inter:bold', 'redirected'),
			refused('gf:inter:italic', 'redirected')
		]);

		expect(note).toBe(
			'FontNest kept 1 file because it is no longer in your font folder and 2 files because the file has been linked somewhere else.'
		);
	});
});
