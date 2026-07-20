import type { FontFaceSummary } from '$lib/bindings/FontFaceSummary';
import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
import { formatCodepoint } from '$lib/glyph-categories';

import { tidyEntries, type ContextMenuEntry, type ContextMenuRequest } from './types';

/** A CSS `font-family` value the user can paste straight into a stylesheet. */
export function cssFontStack(familyName: string): string {
	return `font-family: "${escapeCssString(familyName)}", system-ui, sans-serif;`;
}

/** The full rule for one face: family, weight, and posture together. */
export function cssFaceDeclaration(familyName: string, face: FontFaceSummary): string {
	const style = face.style === 'normal' ? 'normal' : face.style;
	return [
		`font-family: "${escapeCssString(familyName)}";`,
		`font-weight: ${face.weight};`,
		`font-style: ${style};`
	].join('\n');
}

/** `&#x41;` — the HTML numeric character reference for a codepoint. */
export function htmlCharacterReference(codepoint: number): string {
	return `&#x${codepoint.toString(16).toUpperCase()};`;
}

/** `\0041` — the CSS escape, padded to six digits so a following digit cannot join it. */
export function cssCharacterEscape(codepoint: number): string {
	return `\\${codepoint.toString(16).toUpperCase().padStart(6, '0')}`;
}

function escapeCssString(value: string): string {
	return value.replace(/["\\]/g, '\\$&');
}

export type CopyHandler = (label: string, value: string) => void;

export type FamilyMenuOptions = {
	family: FontFamilySummary;
	expanded: boolean;
	pinned: boolean;
	/** False in the browser dev server, where no file manager is reachable. */
	native: boolean;
	onToggleExpanded: () => void;
	onOpenPreview: () => void;
	onClosePreview: () => void;
	onReviewConflict: () => void;
	onUseAsPreviewText: () => void;
	onRevealFile: () => void;
	onCopyFilePath: () => void;
	onCopy: CopyHandler;
};

export function familyContextMenu(options: FamilyMenuOptions): ContextMenuRequest {
	const { family } = options;
	const entries: ContextMenuEntry[] = [
		{
			kind: 'action',
			id: 'toggle',
			label: options.expanded ? 'Close family' : 'Open family',
			icon: 'chevron',
			run: options.onToggleExpanded
		},
		{
			kind: 'action',
			id: 'preview',
			label: options.pinned ? 'Open saved preview' : 'Save and preview',
			icon: 'bookmark',
			run: options.onOpenPreview
		}
	];

	if (options.pinned) {
		entries.push({
			kind: 'action',
			id: 'close-preview',
			label: 'Remove from saved previews',
			icon: 'close',
			run: options.onClosePreview
		});
	}

	if (family.hasConflict) {
		entries.push({ kind: 'separator', id: 'conflict-rule' });
		entries.push({
			kind: 'action',
			id: 'conflict',
			label: 'Review conflict',
			icon: 'alert',
			hint: `${family.fileCount} files`,
			run: options.onReviewConflict
		});
	}

	entries.push({ kind: 'separator', id: 'copy-rule' });
	entries.push(
		{
			kind: 'action',
			id: 'copy-name',
			label: 'Copy family name',
			icon: 'copy',
			run: () => options.onCopy('Family name', family.name)
		},
		{
			kind: 'action',
			id: 'copy-css',
			label: 'Copy CSS font stack',
			icon: 'copy',
			run: () => options.onCopy('CSS font stack', cssFontStack(family.name))
		},
		{
			kind: 'action',
			id: 'preview-text',
			label: 'Use name as preview text',
			icon: 'font',
			run: options.onUseAsPreviewText
		}
	);

	const firstFace = family.faces[0];
	if (options.native && firstFace) {
		entries.push({ kind: 'separator', id: 'file-rule' });
		entries.push(
			{
				kind: 'action',
				id: 'reveal',
				label: 'Show in file manager',
				icon: 'folder',
				hint: firstFace.fileName,
				run: options.onRevealFile
			},
			{
				kind: 'action',
				id: 'copy-path',
				label: 'Copy file path',
				icon: 'copy',
				run: options.onCopyFilePath
			}
		);
	}

	return {
		title: family.name,
		subtitle: `${family.faceCount} ${family.faceCount === 1 ? 'style' : 'styles'} · ${family.formats.join(' · ')}`,
		entries: tidyEntries(entries)
	};
}

export type FaceMenuOptions = {
	familyName: string;
	face: FontFaceSummary;
	native: boolean;
	onRevealFile: () => void;
	onCopyFilePath: () => void;
	onCopy: CopyHandler;
};

export function faceContextMenu(options: FaceMenuOptions): ContextMenuRequest {
	const { face } = options;
	const entries: ContextMenuEntry[] = [
		{
			kind: 'action',
			id: 'copy-postscript',
			label: 'Copy PostScript name',
			icon: 'copy',
			hint: face.postScriptName,
			run: () => options.onCopy('PostScript name', face.postScriptName)
		},
		{
			kind: 'action',
			id: 'copy-style',
			label: 'Copy style name',
			icon: 'copy',
			run: () => options.onCopy('Style name', face.styleName)
		},
		{
			kind: 'action',
			id: 'copy-declaration',
			label: 'Copy CSS declaration',
			icon: 'copy',
			run: () =>
				options.onCopy('CSS declaration', cssFaceDeclaration(options.familyName, face))
		},
		{
			kind: 'action',
			id: 'copy-file-name',
			label: 'Copy file name',
			icon: 'copy',
			hint: face.fileName,
			run: () => options.onCopy('File name', face.fileName)
		}
	];

	if (options.native) {
		entries.push({ kind: 'separator', id: 'file-rule' });
		entries.push(
			{
				kind: 'action',
				id: 'reveal',
				label: 'Show in file manager',
				icon: 'folder',
				run: options.onRevealFile
			},
			{
				kind: 'action',
				id: 'copy-path',
				label: 'Copy file path',
				icon: 'copy',
				run: options.onCopyFilePath
			}
		);
	}

	return {
		title: `${options.familyName} ${face.styleName}`,
		subtitle: `${face.weight} · ${face.format}`,
		entries: tidyEntries(entries)
	};
}

export type DiscoverFamilyMenuOptions = {
	familyName: string;
	category: string;
	license: string;
	installed: boolean;
	expanded: boolean;
	onToggleExpanded: () => void;
	onUseAsPreviewText: () => void;
	onCopy: CopyHandler;
};

export function discoverFamilyContextMenu(options: DiscoverFamilyMenuOptions): ContextMenuRequest {
	return {
		title: options.familyName,
		subtitle: options.installed ? 'Managed by FontNest' : `${options.category} · available`,
		entries: tidyEntries([
			{
				kind: 'action',
				id: 'toggle',
				label: options.expanded ? 'Close details' : 'Open family',
				icon: 'chevron',
				run: options.onToggleExpanded
			},
			{ kind: 'separator', id: 'copy-rule' },
			{
				kind: 'action',
				id: 'copy-name',
				label: 'Copy family name',
				icon: 'copy',
				run: () => options.onCopy('Family name', options.familyName)
			},
			{
				kind: 'action',
				id: 'copy-css',
				label: 'Copy CSS font stack',
				icon: 'copy',
				run: () => options.onCopy('CSS font stack', cssFontStack(options.familyName))
			},
			{
				kind: 'action',
				id: 'copy-license',
				label: 'Copy license',
				icon: 'copy',
				hint: options.license,
				run: () => options.onCopy('License', options.license)
			},
			{
				kind: 'action',
				id: 'preview-text',
				label: 'Use name as preview text',
				icon: 'font',
				run: options.onUseAsPreviewText
			}
		])
	};
}

export type SavedPreviewMenuOptions = {
	familyName: string;
	active: boolean;
	onOpen: () => void;
	onClose: () => void;
	onCopy: CopyHandler;
};

export function savedPreviewContextMenu(options: SavedPreviewMenuOptions): ContextMenuRequest {
	return {
		title: options.familyName,
		subtitle: 'Saved preview',
		entries: tidyEntries([
			{
				kind: 'action',
				id: 'open',
				label: options.active ? 'Already open' : 'Open preview',
				icon: 'font',
				disabled: options.active,
				run: options.onOpen
			},
			{
				kind: 'action',
				id: 'close',
				label: 'Remove from saved previews',
				icon: 'close',
				run: options.onClose
			},
			{ kind: 'separator', id: 'copy-rule' },
			{
				kind: 'action',
				id: 'copy-name',
				label: 'Copy family name',
				icon: 'copy',
				run: () => options.onCopy('Family name', options.familyName)
			}
		])
	};
}

export type GlyphMenuOptions = {
	codepoint: number;
	/**
	 * True when the glyph is already held in the large preview. Omit `onToggleLock`
	 * entirely on surfaces that have no such preview, such as the library sample row.
	 */
	locked?: boolean;
	onToggleLock?: () => void;
	onAppendToPreviewText: () => void;
	onCopy: CopyHandler;
};

export function glyphContextMenu(options: GlyphMenuOptions): ContextMenuRequest {
	const character = String.fromCodePoint(options.codepoint);
	const label = formatCodepoint(options.codepoint);
	const entries: ContextMenuEntry[] = [];

	if (options.onToggleLock) {
		entries.push({
			kind: 'action',
			id: 'lock',
			label: options.locked ? 'Unlock from preview' : 'Lock in preview',
			icon: 'bookmark',
			run: options.onToggleLock
		});
	}

	return {
		title: label,
		subtitle: 'Character',
		entries: tidyEntries([
			...entries,
			{
				kind: 'action',
				id: 'append',
				label: 'Add to preview text',
				icon: 'font',
				run: options.onAppendToPreviewText
			},
			{ kind: 'separator', id: 'copy-rule' },
			{
				kind: 'action',
				id: 'copy-character',
				label: 'Copy character',
				icon: 'copy',
				run: () => options.onCopy('Character', character)
			},
			{
				kind: 'action',
				id: 'copy-codepoint',
				label: 'Copy codepoint',
				icon: 'copy',
				hint: label,
				run: () => options.onCopy('Codepoint', label)
			},
			{
				kind: 'action',
				id: 'copy-html',
				label: 'Copy HTML entity',
				icon: 'copy',
				hint: htmlCharacterReference(options.codepoint),
				run: () => options.onCopy('HTML entity', htmlCharacterReference(options.codepoint))
			},
			{
				kind: 'action',
				id: 'copy-css',
				label: 'Copy CSS escape',
				icon: 'copy',
				hint: cssCharacterEscape(options.codepoint),
				run: () => options.onCopy('CSS escape', cssCharacterEscape(options.codepoint))
			}
		])
	};
}
