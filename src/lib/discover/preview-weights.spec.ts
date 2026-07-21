import { describe, expect, it } from 'vitest';

import { hasWeightAxis, pickPreviewArtifact, staticStyleWeight } from './preview-weights';

describe('hasWeightAxis', () => {
	it('reads the axis list the manifest puts in the style name', () => {
		expect(hasWeightAxis('Variable (wght)')).toBe(true);
		expect(hasWeightAxis('Variable (opsz,wght)')).toBe(true);
		expect(hasWeightAxis('Variable Italic (wdth,wght)')).toBe(true);
		expect(hasWeightAxis('Variable (opsz)')).toBe(false);
		expect(hasWeightAxis('Variable (XROT,YROT)')).toBe(false);
		expect(hasWeightAxis('Bold')).toBe(false);
	});
});

describe('staticStyleWeight', () => {
	it('maps style names to weights and ignores variable and italic files', () => {
		expect(staticStyleWeight('Thin')).toBe(100);
		expect(staticStyleWeight('ExtraLight')).toBe(200);
		expect(staticStyleWeight('Regular')).toBe(400);
		expect(staticStyleWeight('SemiBold')).toBe(600);
		expect(staticStyleWeight('Black')).toBe(900);
		expect(staticStyleWeight('Heavy')).toBe(900);
		expect(staticStyleWeight('BoldItalic')).toBeNull();
		expect(staticStyleWeight('Variable (wght)')).toBeNull();
		expect(staticStyleWeight('Cursive')).toBeNull();
	});

	it('looks past the vendor prefixes the manifest carries', () => {
		expect(staticStyleWeight('Web Bold')).toBe(700);
		expect(staticStyleWeight('Caption Web Regular')).toBe(400);
	});
});

describe('pickPreviewArtifact', () => {
	const variableFamily = [
		{ id: 'gf:inter:regular', style: 'Variable (opsz,wght)' },
		{ id: 'gf:inter:italic', style: 'Variable Italic (opsz,wght)' }
	];
	const staticFamily = [
		{ id: 'gf:roboto:light', style: 'Light' },
		{ id: 'gf:roboto:regular', style: 'Regular' },
		{ id: 'gf:roboto:bold', style: 'Bold' },
		{ id: 'gf:roboto:bolditalic', style: 'BoldItalic' }
	];

	it('sends every weight to the one variable file that covers them', () => {
		for (const weight of [100, 400, 900]) {
			expect(pickPreviewArtifact(variableFamily, weight)).toEqual({
				artifactId: 'gf:inter:regular',
				weight: null,
				variable: true
			});
		}
	});

	it('fetches the closest static cut when the family ships one file per weight', () => {
		expect(pickPreviewArtifact(staticFamily, 700)?.artifactId).toBe('gf:roboto:bold');
		expect(pickPreviewArtifact(staticFamily, 300)?.artifactId).toBe('gf:roboto:light');
		expect(pickPreviewArtifact(staticFamily, 900)?.artifactId).toBe('gf:roboto:bold');
		expect(pickPreviewArtifact(staticFamily, 500)).toEqual({
			artifactId: 'gf:roboto:regular',
			weight: 400,
			variable: false
		});
	});

	it('breaks a tie towards the heavier file so the specimen never reads lighter than asked', () => {
		const family = [
			{ id: 'gf:x:medium', style: 'Medium' },
			{ id: 'gf:x:semibold', style: 'SemiBold' }
		];
		expect(pickPreviewArtifact(family, 550)?.artifactId).toBe('gf:x:semibold');
	});

	it('treats a variable file without a weight axis as the fixed weight it is', () => {
		const family = [
			{ id: 'gf:y:variable', style: 'Variable (opsz)' },
			{ id: 'gf:y:regular', style: 'Regular' }
		];
		expect(pickPreviewArtifact(family, 700)).toEqual({
			artifactId: 'gf:y:regular',
			weight: 400,
			variable: false
		});
	});

	it('never picks an italic file for an upright specimen', () => {
		expect(pickPreviewArtifact([{ id: 'gf:z:italic', style: 'Italic' }], 400)).toBeNull();
	});
});
