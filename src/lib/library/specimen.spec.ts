import { describe, expect, it } from 'vitest';

import type { FontFaceSummary } from '$lib/bindings/FontFaceSummary';
import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';

import {
	drawnWeight,
	faceDetail,
	faceSpecimenStyle,
	familyPreviewStyle,
	safeFontStack,
	specimenText,
	substituteWeightNote,
	weightRange
} from './specimen';

function family(overrides: Partial<FontFamilySummary> = {}): FontFamilySummary {
	return {
		id: 'family:Inter',
		name: 'Inter',
		faceCount: 2,
		fileCount: 2,
		styles: ['Regular'],
		weights: [400, 700],
		formats: ['TrueType'],
		origins: ['userInstalled'],
		monospaced: false,
		variable: false,
		hasConflict: false,
		faces: [],
		...overrides
	};
}

function face(overrides: Partial<FontFaceSummary> = {}): FontFaceSummary {
	return {
		id: 'face:Inter-Regular',
		postScriptName: 'Inter-Regular',
		styleName: 'Regular',
		style: 'Regular',
		weight: 400,
		format: 'TrueType',
		origin: 'userInstalled',
		fileName: 'Inter-Regular.ttf',
		faceIndex: 0,
		monospaced: false,
		variable: false,
		...overrides
	};
}

describe('putting a family name into CSS', () => {
	it('quotes the name and keeps a fallback behind it', () => {
		expect(safeFontStack('Inter')).toBe('"Inter", system-ui, sans-serif');
	});

	// Family names come from font files, so they are not ours to trust. A name holding a quote or
	// a semicolon would close the declaration and let the rest be read as more CSS.
	it('takes out the characters that would end the string or the declaration', () => {
		expect(safeFontStack('Evil"; color: red; font-family: "x')).toBe(
			'"Evil color: red font-family: x", system-ui, sans-serif'
		);
		expect(safeFontStack('Back\\slash')).toBe('"Backslash", system-ui, sans-serif');
		expect(safeFontStack('Two\nLines\r')).toBe('"TwoLines", system-ui, sans-serif');
	});

	it('leaves an ordinary name with spaces and punctuation alone', () => {
		expect(safeFontStack('Noto Sans JP')).toBe('"Noto Sans JP", system-ui, sans-serif');
	});
});

describe('choosing the weight a row is drawn at', () => {
	// A variable family covers the range from one file, so the asked-for weight goes straight
	// through; the catalogue only lists its named instances, often just 400.
	it('passes the asked-for weight straight to a variable family', () => {
		expect(drawnWeight(family({ variable: true, weights: [400] }), 650)).toBe(650);
	});

	it('lands a static family on the closest cut it owns', () => {
		expect(drawnWeight(family({ weights: [400, 700] }), 650)).toBe(700);
	});

	it('says nothing when the family can be drawn at the weight asked for', () => {
		expect(substituteWeightNote(family({ weights: [400, 700] }), 700)).toBeNull();
		expect(substituteWeightNote(family({ variable: true, weights: [400] }), 650)).toBeNull();
	});

	// A row that cannot follow the slider otherwise just looks stuck.
	it('names the cut it fell back to when it cannot follow the slider', () => {
		expect(substituteWeightNote(family({ weights: [400, 700] }), 650)).toBe(
			'Closest cut: Bold'
		);
	});

	it('draws the row in the family it belongs to, at the weight it settled on', () => {
		expect(familyPreviewStyle(family({ weights: [400, 700] }), 650)).toBe(
			'font-family: "Inter", system-ui, sans-serif; font-weight: 700;'
		);
	});
});

describe('drawing one face', () => {
	it('uses the face own weight and slant', () => {
		expect(faceSpecimenStyle(family(), 700, 'italic')).toContain('font-style: italic');
		expect(faceSpecimenStyle(family(), 700, 'Regular')).toContain('font-style: normal');
	});

	// Anything that is not italic is upright. A style name the parser did not recognise should
	// not tip the specimen into a slant.
	it('treats any style that is not italic as upright', () => {
		expect(faceSpecimenStyle(family(), 400, 'oblique')).toContain('font-style: normal');
	});

	it('names the file, and the origin only when the family has more than one', () => {
		expect(faceDetail(family(), face())).toBe('Inter-Regular.ttf');
		expect(faceDetail(family({ origins: ['userInstalled', 'systemDefault'] }), face())).toBe(
			'Inter-Regular.ttf · Installed'
		);
	});

	it('marks a variable face as variable', () => {
		expect(faceDetail(family(), face({ variable: true }))).toBe('Inter-Regular.ttf · Variable');
	});
});

describe('what a specimen row says', () => {
	it('shows the family name when that is the mode', () => {
		expect(specimenText(family(), 'names', 'Hamburgefonstiv')).toBe('Inter');
	});

	it('shows what was typed once there is anything in it', () => {
		expect(specimenText(family(), 'custom', 'Hamburgefonstiv')).toBe('Hamburgefonstiv');
	});

	// An empty box should leave the rows readable rather than blank.
	it('falls back to the family name when the box has been emptied', () => {
		expect(specimenText(family(), 'custom', '   ')).toBe('Inter');
	});
});

describe('saying how far a family reaches', () => {
	it('gives a range for a family with several weights', () => {
		expect(weightRange(family({ weights: [400, 700] }))).toBe('Weights 400–700');
	});

	it('names the single weight a one-cut family has', () => {
		expect(weightRange(family({ weights: [400] }))).toBe('Weight Regular 400');
	});

	it('marks a variable range as variable', () => {
		expect(weightRange(family({ weights: [100, 900], variable: true }))).toBe(
			'Weights 100–900 (variable)'
		);
	});

	it('says nothing at all about a family listing no weights', () => {
		expect(weightRange(family({ weights: [] }))).toBeNull();
	});
});
