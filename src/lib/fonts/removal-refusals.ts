import type { FontRemovalRefusal } from '$lib/bindings/FontRemovalRefusal';
import type { RefusedFontRemoval } from '$lib/bindings/RefusedFontRemoval';

/**
 * What to tell someone about a font FontNest would not remove.
 *
 * The backend proves ownership of every file before it takes one away, and a font that fails a
 * check is left exactly as it was. Saying which check stopped matters: a font somebody replaced
 * needs a different response from a font that is no longer where it was installed.
 */
const REFUSAL_NOTES: Record<FontRemovalRefusal, string> = {
	unknownSource: 'this version of FontNest no longer knows where it came from',
	locationUnavailable: 'your Windows font folder could not be found',
	recordMismatch: 'its record no longer matches where it was installed',
	missing: 'it is no longer in your font folder',
	redirected: 'the file has been linked somewhere else',
	outsideFontFolder: 'it now sits outside your font folder',
	changed: 'the file has changed since FontNest installed it',
	notRegistered: 'Windows no longer has it registered to that file',
	protected: 'Windows protects it',
	unreadable: 'the file could not be read'
};

/**
 * One sentence covering every font a removal left alone, or an empty string when it left none.
 *
 * Refusals are grouped by reason so a family with twelve untouched files reads as one sentence
 * rather than twelve.
 */
export function removalRefusalNote(refused: RefusedFontRemoval[]): string {
	if (!refused.length) return '';

	const counts = new Map<FontRemovalRefusal, number>();
	for (const font of refused) counts.set(font.reason, (counts.get(font.reason) ?? 0) + 1);

	const reasons = [...counts.entries()].map(([reason, count]) => {
		const files = count === 1 ? '1 file' : `${count} files`;
		return `${files} because ${REFUSAL_NOTES[reason]}`;
	});
	const joined =
		reasons.length === 1
			? reasons[0]
			: `${reasons.slice(0, -1).join(', ')} and ${reasons[reasons.length - 1]}`;

	return `FontNest kept ${joined}.`;
}
