import { describe, expect, it } from 'vitest';

import { getDirectionalReorderPosition, reorderIds } from './reorder';

describe('getDirectionalReorderPosition', () => {
	it('inserts after the target while moving down', () => {
		expect(getDirectionalReorderPosition(['a', 'b'], 'a', 'b')).toBe('after');
	});

	it('inserts before the target while moving up', () => {
		expect(getDirectionalReorderPosition(['a', 'b'], 'b', 'a')).toBe('before');
	});

	it('rejects no-op and invalid targets', () => {
		expect(getDirectionalReorderPosition(['a', 'b'], 'a', 'a')).toBeNull();
		expect(getDirectionalReorderPosition(['a', 'b'], 'a', 'missing')).toBeNull();
	});
});

describe('reorderIds', () => {
	it('moves an item before a target', () => {
		expect(reorderIds(['a', 'b', 'c'], 'c', 'a', 'before')).toEqual(['c', 'a', 'b']);
	});

	it('moves an item after a target', () => {
		expect(reorderIds(['a', 'b', 'c'], 'a', 'c', 'after')).toEqual(['b', 'c', 'a']);
	});

	it('leaves the order unchanged for invalid moves', () => {
		expect(reorderIds(['a', 'b', 'c'], 'a', 'a', 'after')).toEqual(['a', 'b', 'c']);
		expect(reorderIds(['a', 'b', 'c'], 'missing', 'b', 'before')).toEqual(['a', 'b', 'c']);
	});
});
