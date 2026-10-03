import type { FontRemovalRefusal } from '$lib/bindings/FontRemovalRefusal';
import type { ManagedFontInventory } from '$lib/bindings/ManagedFontInventory';
import type { ManagedFontSummary } from '$lib/bindings/ManagedFontSummary';
import type { QuarantinedFontSummary } from '$lib/bindings/QuarantinedFontSummary';

/**
 * Describing the fonts FontNest put on this computer, and the ones it has taken back off.
 *
 * Removal never deletes: the file is moved into FontNest's own data and the ledger records where
 * it went, so a removal can be undone. That is only worth anything if the interface says so, which
 * is most of what this module is for.
 */

export const EMPTY_INVENTORY: ManagedFontInventory = { installed: [], quarantined: [] };

/** Where a font came from, in the words the interface uses for it elsewhere. */
export function sourceLabel(font: ManagedFontSummary | QuarantinedFontSummary): string {
	return font.imported ? 'Imported' : 'Discover';
}

/** Why a removal left a font alone, said plainly rather than as a refusal code. */
export function refusalDetail(reason: FontRemovalRefusal): string {
	switch (reason) {
		case 'unknownSource':
			return 'This version of FontNest does not recognise where that font came from.';
		case 'locationUnavailable':
			return 'Your personal font folder could not be found.';
		case 'recordMismatch':
			return 'The record of that font does not match where it actually is, so FontNest left it alone.';
		case 'missing':
			return 'The font is no longer where FontNest installed it.';
		case 'redirected':
			return 'That path is a link rather than the installed file, so following it would have reached somewhere else.';
		case 'outsideFontFolder':
			return 'The file sits outside your personal font folder, so it is not FontNest’s to remove.';
		case 'changed':
			return 'The file no longer holds the bytes FontNest installed, so it is a different font now.';
		case 'notRegistered':
			return 'Windows no longer has that font registered to this file.';
		case 'protected':
			return 'Windows protects that font.';
		case 'unreadable':
			return 'The font file could not be read.';
		default:
			return 'FontNest could not prove that font is one it installed.';
	}
}

/** A calm line for how long ago a removal happened. */
export function removedAgo(removedAt: number, now: number = Date.now()): string {
	const seconds = Math.max(0, Math.round(now / 1000 - removedAt));
	if (seconds < 90) return 'just now';

	const minutes = Math.round(seconds / 60);
	if (minutes < 60) return `${minutes} minutes ago`;

	const hours = Math.round(minutes / 60);
	if (hours < 24) return hours === 1 ? 'an hour ago' : `${hours} hours ago`;

	const days = Math.round(hours / 24);
	return days === 1 ? 'yesterday' : `${days} days ago`;
}

/** What the section says about itself before anyone reads the list. */
export function inventorySummary(inventory: ManagedFontInventory): string {
	const { installed, quarantined } = inventory;
	if (installed.length === 0 && quarantined.length === 0) {
		return 'FontNest has not installed any fonts on this computer.';
	}

	const fonts = installed.length === 1 ? '1 font' : `${installed.length} fonts`;
	if (quarantined.length === 0) return `${fonts} installed by FontNest.`;
	const aside = quarantined.length === 1 ? '1 more' : `${quarantined.length} more`;
	return `${fonts} installed by FontNest, and ${aside} set aside.`;
}

/** Installed fonts grouped by family, so a family with several styles reads as one thing. */
export function byFamily(installed: ManagedFontSummary[]): [string, ManagedFontSummary[]][] {
	const families = new Map<string, ManagedFontSummary[]>();
	for (const font of installed) {
		const existing = families.get(font.familyName);
		if (existing) existing.push(font);
		else families.set(font.familyName, [font]);
	}
	return [...families.entries()];
}

/** A size in the units a person reads, for space they are deciding whether to reclaim. */
export function formatBytes(bytes: number): string {
	if (bytes <= 0) return '0 KB';
	if (bytes < 1024) return '1 KB';
	if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** What clearing everything set aside would free. */
export function setAsideBytes(quarantined: QuarantinedFontSummary[]): number {
	return quarantined.reduce((total, font) => total + font.sizeBytes, 0);
}
