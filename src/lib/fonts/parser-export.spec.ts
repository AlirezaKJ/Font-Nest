import { describe, expect, it } from 'vitest';

import type { FontParserJsonEvent } from '$lib/bindings/FontParserJsonEvent';
import {
	applyParserJsonEvent,
	parserExportProgress,
	parserExportText,
	parserExportTruncationNote,
	type ParserExport
} from './parser-export';

const started: FontParserJsonEvent = {
	event: 'started',
	data: {
		faceId: 'face:0123456789abcdef0123456789abcdef01234567',
		parserName: 'ttf-parser',
		parserVersion: '0.25.1',
		totalBytes: 18,
		chunkCount: 2,
		truncated: false,
		unicodeMappings: { included: 12, total: 12 },
		glyphs: { included: 40, total: 40 }
	}
};

function stream(...events: FontParserJsonEvent[]): ParserExport | null {
	return events.reduce<ParserExport | null>(
		(current, event) => applyParserJsonEvent(current, event),
		null
	);
}

describe('applyParserJsonEvent', () => {
	it('assembles ordered chunks into one document', () => {
		const exported = stream(
			started,
			{ event: 'chunk', data: { index: 0, text: '{"parser":' } },
			{ event: 'chunk', data: { index: 1, text: '{}}' } },
			{ event: 'finished' }
		);

		expect(parserExportText(exported)).toBe('{"parser":{}}');
		expect(exported?.complete).toBe(true);
		expect(parserExportProgress(exported)).toBe(1);
	});

	it('reassembles chunks by index even when they arrive out of order', () => {
		const exported = stream(
			started,
			{ event: 'chunk', data: { index: 1, text: '{}}' } },
			{ event: 'chunk', data: { index: 0, text: '{"parser":' } }
		);

		expect(parserExportText(exported)).toBe('{"parser":{}}');
	});

	it('reports partial progress before the last chunk lands', () => {
		const exported = stream(started, { event: 'chunk', data: { index: 0, text: 'half' } });

		expect(exported?.complete).toBe(false);
		expect(parserExportProgress(exported)).toBe(0.5);
	});

	it('marks a cancelled export and ignores chunks that arrive after it', () => {
		const exported = stream(
			started,
			{ event: 'chunk', data: { index: 0, text: 'half' } },
			{ event: 'cancelled' },
			{ event: 'chunk', data: { index: 1, text: 'late' } }
		);

		expect(exported?.cancelled).toBe(true);
		expect(parserExportText(exported)).toBe('half');
	});

	it('starts a fresh export rather than appending to the previous one', () => {
		const first = stream(started, { event: 'chunk', data: { index: 0, text: 'old' } });
		const second = applyParserJsonEvent(first, started);

		expect(second?.chunks).toEqual([]);
	});

	it('ignores chunks that arrive before an export has started', () => {
		expect(stream({ event: 'chunk', data: { index: 0, text: 'orphan' } })).toBeNull();
	});
});

describe('parserExportTruncationNote', () => {
	it('says nothing when the snapshot holds the whole face', () => {
		expect(parserExportTruncationNote(stream(started))).toBe('');
	});

	it('states what a capped snapshot left out', () => {
		const note = parserExportTruncationNote(
			stream({
				...started,
				data: {
					...started.data,
					truncated: true,
					glyphs: { included: 4096, total: 65535 },
					unicodeMappings: { included: 4096, total: 28000 }
				}
			} as FontParserJsonEvent)
		);

		expect(note).toContain('4,096 of 65,535 glyphs');
		expect(note).toContain('4,096 of 28,000 Unicode mappings');
	});
});
