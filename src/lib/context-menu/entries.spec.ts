import { describe, expect, it, vi } from 'vitest';

import type { FontFaceSummary } from '$lib/bindings/FontFaceSummary';
import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';

import {
	cssCharacterEscape,
	cssFaceDeclaration,
	cssFontStack,
	faceContextMenu,
	familyContextMenu,
	glyphContextMenu,
	htmlCharacterReference
} from './entries';
import { isAction, tidyEntries, type ContextMenuEntry } from './types';

const FACE: FontFaceSummary = {
	id: 'face:0000000000000000000000000000000000000000',
	postScriptName: 'SourceSerif4-Bold',
	styleName: 'Bold',
	style: 'normal',
	weight: 700,
	format: 'OpenType',
	origin: 'userInstalled',
	fileName: 'SourceSerif4-Bold.otf',
	faceIndex: 0,
	monospaced: false,
	variable: false
};

const FAMILY: FontFamilySummary = {
	id: 'source serif 4',
	name: 'Source Serif 4',
	faceCount: 2,
	fileCount: 2,
	styles: ['Regular', 'Bold'],
	weights: [400, 700],
	formats: ['OpenType'],
	origins: ['userInstalled'],
	monospaced: false,
	variable: false,
	hasConflict: false,
	faces: [FACE]
};

function noop() {}

function familyOptions(family: FontFamilySummary, native: boolean) {
	return {
		family,
		expanded: false,
		pinned: false,
		native,
		onToggleExpanded: noop,
		onOpenPreview: noop,
		onClosePreview: noop,
		onReviewConflict: noop,
		onUseAsPreviewText: noop,
		onRevealFile: noop,
		onCopyFilePath: noop,
		onCopy: noop
	};
}

function actionIds(entries: ContextMenuEntry[]): string[] {
	return entries.filter(isAction).map((entry) => entry.id);
}

describe('pasteable font values', () => {
	it('quotes and escapes the family name in a CSS stack', () => {
		expect(cssFontStack('Source Serif 4')).toBe(
			'font-family: "Source Serif 4", system-ui, sans-serif;'
		);
		expect(cssFontStack('He said "Hi"')).toContain('\\"Hi\\"');
	});

	it('writes a full declaration for one face', () => {
		expect(cssFaceDeclaration('Source Serif 4', FACE)).toBe(
			'font-family: "Source Serif 4";\nfont-weight: 700;\nfont-style: normal;'
		);
	});

	it('pads the CSS escape so a following digit cannot join it', () => {
		expect(cssCharacterEscape(0x41)).toBe('\\000041');
		expect(htmlCharacterReference(0x41)).toBe('&#x41;');
	});
});

describe('family context menu', () => {
	it('offers conflict review only for families that have one', () => {
		expect(actionIds(familyContextMenu(familyOptions(FAMILY, true)).entries)).not.toContain(
			'conflict'
		);
		expect(
			actionIds(
				familyContextMenu(familyOptions({ ...FAMILY, hasConflict: true }, true)).entries
			)
		).toContain('conflict');
	});

	it('hides filesystem actions when no file manager is reachable', () => {
		const ids = actionIds(familyContextMenu(familyOptions(FAMILY, false)).entries);

		expect(ids).not.toContain('reveal');
		expect(ids).not.toContain('copy-path');
	});

	it('copies the family name through the shared copy handler', () => {
		const onCopy = vi.fn();
		const menu = familyContextMenu({ ...familyOptions(FAMILY, true), onCopy });
		const copyName = menu.entries.filter(isAction).find((entry) => entry.id === 'copy-name');

		copyName?.run();

		expect(onCopy).toHaveBeenCalledWith('Family name', 'Source Serif 4');
	});

	it('names the family and its shape in the header', () => {
		const menu = familyContextMenu(familyOptions(FAMILY, true));

		expect(menu.title).toBe('Source Serif 4');
		expect(menu.subtitle).toBe('2 styles · OpenType');
	});
});

describe('face context menu', () => {
	it('always offers the four identifiers a designer copies', () => {
		const ids = actionIds(
			faceContextMenu({
				familyName: 'Source Serif 4',
				face: FACE,
				native: false,
				onRevealFile: noop,
				onCopyFilePath: noop,
				onCopy: noop
			}).entries
		);

		expect(ids).toEqual([
			'copy-postscript',
			'copy-style',
			'copy-declaration',
			'copy-file-name'
		]);
	});
});

describe('glyph context menu', () => {
	it('drops the lock action on surfaces with no locked preview', () => {
		const ids = actionIds(
			glyphContextMenu({ codepoint: 0x41, onAppendToPreviewText: noop, onCopy: noop }).entries
		);

		expect(ids).not.toContain('lock');
		expect(ids[0]).toBe('append');
	});

	it('copies the character itself, not its label', () => {
		const onCopy = vi.fn();
		const menu = glyphContextMenu({
			codepoint: 0x2192,
			onAppendToPreviewText: noop,
			onCopy
		});

		menu.entries
			.filter(isAction)
			.find((entry) => entry.id === 'copy-character')
			?.run();

		expect(onCopy).toHaveBeenCalledWith('Character', '→');
		expect(menu.title).toBe('U+2192');
	});
});

describe('entry tidying', () => {
	it('removes leading, trailing, and doubled rules', () => {
		const tidied = tidyEntries([
			{ kind: 'separator', id: 'a' },
			{ kind: 'action', id: 'one', label: 'One', run: noop },
			{ kind: 'separator', id: 'b' },
			{ kind: 'separator', id: 'c' },
			{ kind: 'action', id: 'two', label: 'Two', run: noop },
			{ kind: 'separator', id: 'd' }
		]);

		expect(tidied.map((entry) => entry.id)).toEqual(['one', 'b', 'two']);
	});
});
