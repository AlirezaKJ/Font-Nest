import { afterEach, describe, expect, it, vi } from 'vitest';

import type { ValidatedLocalFont } from '$lib/bindings/ValidatedLocalFont';
import {
	activateLocalFontPreview,
	clearLocalFontPreviews,
	importLocalFontPreview
} from './local-fonts';

class TestFontFace {
	loaded = false;
	constructor(
		readonly family: string,
		readonly source: string
	) {}

	async load(): Promise<TestFontFace> {
		this.loaded = true;
		return this;
	}
}

function validatedFont(overrides: Partial<ValidatedLocalFont> = {}): ValidatedLocalFont {
	return {
		handle: '0123456789abcdef0123456789abcdef01234567',
		previewFamily: 'FontNestPreview-0123456789abcdef',
		previewUrl: 'http://fontnest-preview.localhost/0123456789abcdef0123456789abcdef01234567',
		fileName: 'Sample.ttf',
		format: 'TrueType',
		faceCount: 1,
		faces: [
			{
				faceIndex: 0,
				familyName: 'Sample',
				subfamilyName: 'Regular',
				fullName: 'Sample Regular',
				postScriptName: 'Sample-Regular',
				isVariable: false,
				glyphCount: 256
			}
		],
		...overrides
	};
}

afterEach(() => {
	clearLocalFontPreviews();
	vi.unstubAllGlobals();
});

describe('importLocalFontPreview', () => {
	it('returns null and never validates when the dialog is cancelled', async () => {
		const validate = vi.fn();
		const result = await importLocalFontPreview(async () => null, validate);

		expect(result).toBeNull();
		expect(validate).not.toHaveBeenCalled();
	});

	it('validates the chosen path and loads the returned handle for preview', async () => {
		const add = vi.fn();
		vi.stubGlobal('FontFace', TestFontFace);
		vi.stubGlobal('document', { fonts: { add, delete: vi.fn() } });
		const font = validatedFont();
		const validate = vi.fn(async () => font);

		const result = await importLocalFontPreview(async () => 'C:/Users/me/Sample.ttf', validate);

		expect(validate).toHaveBeenCalledWith('C:/Users/me/Sample.ttf');
		expect(result).toBe(font);
		expect(add).toHaveBeenCalledTimes(1);
		const face = add.mock.calls[0]?.[0] as TestFontFace;
		expect(face.family).toBe(font.previewFamily);
		expect(face.source).toContain(font.previewUrl);
		expect(face.loaded).toBe(true);
	});

	it('registers each preview family only once', async () => {
		const add = vi.fn();
		vi.stubGlobal('FontFace', TestFontFace);
		vi.stubGlobal('document', { fonts: { add, delete: vi.fn() } });
		const font = validatedFont();

		await activateLocalFontPreview(font);
		await activateLocalFontPreview(font);

		expect(add).toHaveBeenCalledTimes(1);
	});
});
