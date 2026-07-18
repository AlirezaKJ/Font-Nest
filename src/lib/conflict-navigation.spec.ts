import { describe, expect, it } from 'vitest';

import { getConflictDestination } from './conflict-navigation';

describe('conflict navigation', () => {
	it('opens the conflicts workspace when review is requested from the Library', () => {
		expect(getConflictDestination('review')).toBe('duplicates');
	});

	it('returns to the Library when a conflict family is inspected', () => {
		expect(getConflictDestination('inspect')).toBe('library');
	});
});
