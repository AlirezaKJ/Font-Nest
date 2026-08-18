import type { FontParserJsonEvent } from '$lib/bindings/FontParserJsonEvent';
import type { FontParserJsonSection } from '$lib/bindings/FontParserJsonSection';

/** How much of the snapshot is shown inline. The rest is still copied in full. */
export const PARSER_PREVIEW_CHARS = 120_000;

export type ParserExport = {
	faceId: string;
	parserName: string;
	parserVersion: string;
	totalBytes: number;
	chunkCount: number;
	truncated: boolean;
	unicodeMappings: FontParserJsonSection;
	glyphs: FontParserJsonSection;
	chunks: string[];
	complete: boolean;
	cancelled: boolean;
};

/**
 * Folds one streamed export message into the export being assembled.
 *
 * The snapshot arrives in ordered chunks, so this is deliberately additive: it never
 * rewrites earlier chunks, and it ignores messages for an export that was replaced (a
 * different face) so a late chunk from an abandoned run cannot corrupt the current one.
 */
export function applyParserJsonEvent(
	current: ParserExport | null,
	event: FontParserJsonEvent
): ParserExport | null {
	switch (event.event) {
		case 'started':
			return {
				faceId: event.data.faceId,
				parserName: event.data.parserName,
				parserVersion: event.data.parserVersion,
				totalBytes: event.data.totalBytes,
				chunkCount: event.data.chunkCount,
				truncated: event.data.truncated,
				unicodeMappings: event.data.unicodeMappings,
				glyphs: event.data.glyphs,
				chunks: [],
				complete: false,
				cancelled: false
			};
		case 'chunk': {
			if (!current || current.complete || current.cancelled) return current;
			const chunks = [...current.chunks];
			chunks[event.data.index] = event.data.text;
			return { ...current, chunks };
		}
		case 'finished':
			return current ? { ...current, complete: true } : current;
		case 'cancelled':
			return current ? { ...current, cancelled: true } : current;
		default:
			return current;
	}
}

/** The assembled document. Incomplete until the last chunk has arrived. */
export function parserExportText(exported: ParserExport | null): string {
	if (!exported) return '';
	return exported.chunks.join('');
}

/** Progress across the streamed chunks, 0 to 1. */
export function parserExportProgress(exported: ParserExport | null): number {
	if (!exported || exported.chunkCount === 0) return 0;
	const received = exported.chunks.filter((chunk) => chunk !== undefined).length;
	return Math.min(1, received / exported.chunkCount);
}

/**
 * Plain-language account of what the snapshot left out, or an empty string when it holds
 * the whole face. Truncation is stated rather than implied.
 */
export function parserExportTruncationNote(exported: ParserExport | null): string {
	if (!exported?.truncated) return '';
	const format = (value: number) => new Intl.NumberFormat().format(value);
	const parts: string[] = [];
	if (exported.glyphs.included < exported.glyphs.total) {
		parts.push(
			`${format(exported.glyphs.included)} of ${format(exported.glyphs.total)} glyphs`
		);
	}
	if (exported.unicodeMappings.included < exported.unicodeMappings.total) {
		parts.push(
			`${format(exported.unicodeMappings.included)} of ${format(
				exported.unicodeMappings.total
			)} Unicode mappings`
		);
	}
	if (parts.length === 0) return 'This snapshot is a capped sample of the face.';
	return `This snapshot carries ${parts.join(' and ')}.`;
}
