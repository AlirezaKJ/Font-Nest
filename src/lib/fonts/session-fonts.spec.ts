import { afterEach, describe, expect, it, vi } from 'vitest';

import type { GoogleFontArtifactSummary } from '$lib/bindings/GoogleFontArtifactSummary';
import {
	activateInstalledGoogleFont,
	clearSessionFontFaces,
	fontFaceDescriptors
} from './session-fonts';

class TestFontFace {
	constructor(
		readonly family: string,
		readonly source: string,
		readonly descriptors?: FontFaceDescriptors
	) {}

	async load(): Promise<TestFontFace> {
		return this;
	}
}

const artifacts: GoogleFontArtifactSummary[] = [
	{
		id: 'gf:inter:normal',
		fileName: 'Inter[opsz,wght].ttf',
		style: 'Variable (opsz,wght)',
		format: 'TrueType',
		sizeBytes: 100,
		installed: true
	},
	{
		id: 'gf:inter:italic',
		fileName: 'Inter-Italic[opsz,wght].ttf',
		style: 'Variable Italic (opsz,wght)',
		format: 'TrueType',
		sizeBytes: 100,
		installed: true
	}
];

afterEach(() => {
	clearSessionFontFaces();
	vi.unstubAllGlobals();
});

describe('fontFaceDescriptors', () => {
	it('maps static and variable manifest styles to CSS descriptors', () => {
		expect(fontFaceDescriptors('Regular')).toEqual({ style: 'normal', weight: '400' });
		expect(fontFaceDescriptors('SemiBold Italic')).toEqual({ style: 'italic', weight: '600' });
		expect(fontFaceDescriptors('Variable Italic (opsz,wght)')).toEqual({
			style: 'italic',
			weight: '100 900'
		});
	});
});

describe('activateInstalledGoogleFont', () => {
	it('loads verified artifacts under their installed family name for the current webview', async () => {
		const add = vi.fn();
		const remove = vi.fn();
		vi.stubGlobal('FontFace', TestFontFace);
		vi.stubGlobal('document', { fonts: { add, delete: remove } });
		const loadPreview = vi.fn(async (artifactId: string) => ({
			artifactId,
			fontFamily: 'FontNestRemotePreview',
			dataUrl: `data:font/ttf;base64,${artifactId}`
		}));

		await activateInstalledGoogleFont('Inter', artifacts, loadPreview);

		expect(loadPreview).toHaveBeenCalledTimes(2);
		expect(add).toHaveBeenCalledTimes(2);
		expect(add.mock.calls[0]?.[0]).toMatchObject({
			family: 'Inter',
			descriptors: { style: 'normal', weight: '100 900' }
		});
		expect(add.mock.calls[1]?.[0]).toMatchObject({
			family: 'Inter',
			descriptors: { style: 'italic', weight: '100 900' }
		});

		await activateInstalledGoogleFont('Inter', artifacts, loadPreview);
		expect(loadPreview).toHaveBeenCalledTimes(2);
		expect(add).toHaveBeenCalledTimes(2);
	});
});
