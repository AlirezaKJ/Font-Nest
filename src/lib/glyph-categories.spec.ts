import { describe, expect, it } from 'vitest';

import {
	filterGlyphSetCodepoints,
	formatCodepoint,
	glyphCellText,
	groupUnicodeCodepoints
} from './glyph-categories';

describe('groupUnicodeCodepoints', () => {
	it('classifies familiar scripts and symbol series', () => {
		const groups = groupUnicodeCodepoints([
			0x41,
			0x61,
			0x31,
			0x21,
			0x20ac,
			0x2192,
			0x03a9,
			0x0416,
			0x1f600,
			0xe000,
			0x0301
		]);
		const byKey = new Map(groups.map((group) => [group.key, group.codepoints]));

		expect(byKey.get('latin-uppercase')).toEqual([0x41]);
		expect(byKey.get('latin-lowercase')).toEqual([0x61]);
		expect(byKey.get('numbers')).toEqual([0x31]);
		expect(byKey.get('punctuation')).toEqual([0x21]);
		expect(byKey.get('currency')).toEqual([0x20ac]);
		expect(byKey.get('arrows')).toEqual([0x2192]);
		expect(byKey.get('greek')).toEqual([0x03a9]);
		expect(byKey.get('cyrillic')).toEqual([0x0416]);
		expect(byKey.get('emoji-pictographs')).toEqual([0x1f600]);
		expect(byKey.get('private-use')).toEqual([0xe000]);
		expect(byKey.get('combining-marks')).toEqual([0x0301]);
	});

	it('keeps every unique mapped codepoint exactly once', () => {
		const input = [0x61, 0x41, 0x61, 0x20, 0x1f600];
		const flattened = groupUnicodeCodepoints(input)
			.flatMap((group) => group.codepoints)
			.sort((left, right) => left - right);

		expect(flattened).toEqual([0x20, 0x41, 0x61, 0x1f600]);
	});
});

describe('filterGlyphSetCodepoints', () => {
	it('limits the basic set to supported printable Basic Latin characters', () => {
		const input = [0x1f600, 0x7f, 0x7e, 0x41, 0x20, 0x1f, 0x41];

		expect(filterGlyphSetCodepoints(input, 'basic')).toEqual([0x20, 0x41, 0x7e]);
		expect(filterGlyphSetCodepoints(input, 'full')).toEqual([
			0x1f,
			0x20,
			0x41,
			0x7e,
			0x7f,
			0x1f600
		]);
	});
});

describe('glyph cell formatting', () => {
	it('makes combining and non-printing characters inspectable', () => {
		expect(glyphCellText(0x0301)).toBe(`◌${String.fromCodePoint(0x0301)}`);
		expect(glyphCellText(0x20)).toBe('U+0020');
		expect(formatCodepoint(0x1f600)).toBe('U+01F600');
	});
});
