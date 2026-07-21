import { describe, expect, it } from 'vitest';

import { familyOrigin, fontOriginLabel, includesSystemFont, sortOrigins } from './font-origin';

describe('font origin copy', () => {
	it('names each origin in the user’s terms', () => {
		expect(fontOriginLabel('systemDefault')).toBe('System font');
		expect(fontOriginLabel('userInstalled')).toBe('Installed');
		expect(fontOriginLabel('machineInstalled')).toBe('All users');
	});

	it('uses the single origin when a family has only one', () => {
		expect(familyOrigin(['systemDefault']).label).toBe('System font');
	});

	it('calls a family mixed rather than picking one of its origins', () => {
		const mixed = familyOrigin(['userInstalled', 'systemDefault']);

		expect(mixed.label).toBe('Mixed sources');
		expect(mixed.description).toContain('System font');
		expect(mixed.description).toContain('Installed');
	});

	it('falls back to unknown when a family reports no origin', () => {
		expect(familyOrigin([]).label).toBe('Unknown source');
	});

	it('flags families that contain a system font', () => {
		expect(includesSystemFont(['userInstalled', 'systemDefault'])).toBe(true);
		expect(includesSystemFont(['userInstalled'])).toBe(false);
	});

	it('sorts origins from system-owned to user-added', () => {
		expect(sortOrigins(['unknown', 'userInstalled', 'systemDefault'])).toEqual([
			'systemDefault',
			'userInstalled',
			'unknown'
		]);
	});
});
