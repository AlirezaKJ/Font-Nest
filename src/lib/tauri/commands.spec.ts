import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
	cancelFontFaceParserExport,
	checkForAppUpdate,
	exportFontFaceParserJson,
	getGoogleFontDetails,
	inspectFontFace,
	inspectFontGlyphOutline,
	installGoogleFont,
	installAppUpdate,
	listGoogleFonts,
	managedStorageStatus,
	prepareGoogleFontPreview,
	scanInstalledFonts
} from './commands';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn(),
	Channel: class<T> {
		onmessage: (message: T) => void = () => undefined;
	}
}));

describe('scanInstalledFonts', () => {
	beforeEach(() => {
		vi.mocked(invoke).mockReset();
	});

	it('invokes the native catalogue command without frontend-owned paths', async () => {
		const response = {
			families: [],
			familyCount: 0,
			faceCount: 0,
			conflictCount: 0,
			scanDurationMs: 12
		};
		vi.mocked(invoke).mockResolvedValue(response);

		await expect(scanInstalledFonts()).resolves.toEqual(response);
		expect(invoke).toHaveBeenCalledWith('scan_installed_fonts');
	});
});

describe('font face parser commands', () => {
	beforeEach(() => {
		vi.mocked(invoke).mockReset();
	});

	it('inspects faces and glyph outlines through opaque IDs', async () => {
		const faceId = 'face:0123456789abcdef0123456789abcdef01234567';
		vi.mocked(invoke).mockResolvedValueOnce({ faceId, metrics: { unitsPerEm: 1000 } });
		vi.mocked(invoke).mockResolvedValueOnce({ faceId, codepoint: 65, pathData: 'M0 0' });

		await inspectFontFace(faceId);
		await inspectFontGlyphOutline({
			faceId,
			codepoint: 65,
			variations: [{ tag: 'wght', value: 650 }]
		});

		expect(invoke).toHaveBeenNthCalledWith(1, 'inspect_font_face', { faceId });
		expect(invoke).toHaveBeenNthCalledWith(2, 'inspect_font_glyph_outline', {
			request: {
				faceId,
				codepoint: 65,
				variations: [{ tag: 'wght', value: 650 }]
			}
		});
	});

	it('streams a parser snapshot as chunks over a channel', async () => {
		const faceId = 'face:0123456789abcdef0123456789abcdef01234567';
		const events: unknown[] = [];
		vi.mocked(invoke).mockImplementation(async (_command, args) => {
			const channel = (args as { onEvent: { onmessage: (message: unknown) => void } })
				.onEvent;
			channel.onmessage({ event: 'chunk', data: { index: 0, text: '{"parser"' } });
			channel.onmessage({ event: 'finished' });
		});

		await exportFontFaceParserJson({ faceId, exportId: 'export-1' }, (event) =>
			events.push(event)
		);

		expect(invoke).toHaveBeenCalledWith('export_font_face_parser_json', {
			request: { faceId, exportId: 'export-1' },
			onEvent: expect.anything()
		});
		expect(events).toEqual([
			{ event: 'chunk', data: { index: 0, text: '{"parser"' } },
			{ event: 'finished' }
		]);
	});

	it('cancels a running export by its ID', async () => {
		vi.mocked(invoke).mockResolvedValue(undefined);

		await cancelFontFaceParserExport('export-1');

		expect(invoke).toHaveBeenCalledWith('cancel_font_face_parser_export', {
			exportId: 'export-1'
		});
	});
});

describe('Google Fonts commands', () => {
	beforeEach(() => {
		vi.mocked(invoke).mockReset();
	});

	it('lists a bounded catalogue page without exposing source URLs', async () => {
		const response = {
			families: [],
			total: 0,
			offset: 0,
			limit: 60,
			snapshot: 'fixture'
		};
		vi.mocked(invoke).mockResolvedValue(response);
		const request = {
			query: 'serif',
			category: 'serif',
			subset: 'latin',
			technology: 'variable',
			availability: 'available',
			sort: 'recent',
			offset: 0,
			limit: 60
		};

		await expect(listGoogleFonts(request)).resolves.toEqual(response);
		expect(invoke).toHaveBeenCalledWith('list_google_fonts', {
			request
		});
	});

	it('loads details and previews through opaque provider IDs', async () => {
		vi.mocked(invoke).mockResolvedValueOnce({ id: 'gf:inter', artifacts: [] });
		vi.mocked(invoke).mockResolvedValueOnce({
			artifactId: 'gf:inter:regular',
			fontFamily: 'FontNestRemotePreview',
			dataUrl: 'data:font/ttf;base64,AA=='
		});

		await getGoogleFontDetails('gf:inter');
		await prepareGoogleFontPreview('gf:inter:regular');

		expect(invoke).toHaveBeenNthCalledWith(1, 'get_google_font_details', {
			familyId: 'gf:inter'
		});
		expect(invoke).toHaveBeenNthCalledWith(2, 'prepare_google_font_preview', {
			artifactId: 'gf:inter:regular'
		});
	});

	it('installs only explicitly selected manifest artifacts', async () => {
		const response = {
			familyId: 'gf:inter',
			familyName: 'Inter',
			installedArtifactIds: ['gf:inter:regular'],
			alreadyInstalledArtifactIds: []
		};
		vi.mocked(invoke).mockResolvedValue(response);

		await expect(installGoogleFont('gf:inter', ['gf:inter:regular'])).resolves.toEqual(
			response
		);
		expect(invoke).toHaveBeenCalledWith('install_google_font', {
			request: {
				familyId: 'gf:inter',
				artifactIds: ['gf:inter:regular']
			}
		});
	});

	it('reports why managed font operations are unavailable', async () => {
		const response = { writable: false, reason: 'locked' };
		vi.mocked(invoke).mockResolvedValue(response);

		await expect(managedStorageStatus()).resolves.toEqual(response);
		expect(invoke).toHaveBeenCalledWith('managed_storage_status');
	});
});

describe('application updater commands', () => {
	beforeEach(() => {
		vi.mocked(invoke).mockReset();
	});

	it('checks for updates through the Rust-owned updater command', async () => {
		const response = {
			currentVersion: '0.1.0',
			version: '0.1.1',
			notes: 'A safer updater.',
			publishedAt: '2026-07-18 00:00:00 +00:00:00'
		};
		vi.mocked(invoke).mockResolvedValue(response);

		await expect(checkForAppUpdate()).resolves.toEqual(response);
		expect(invoke).toHaveBeenCalledWith('check_for_app_update');
	});

	it('passes only the expected version and a progress channel when installing', async () => {
		const events: unknown[] = [];
		vi.mocked(invoke).mockImplementation(async (_command, args) => {
			const channel = (args as { onEvent: { onmessage: (message: unknown) => void } })
				.onEvent;
			channel.onmessage({
				event: 'downloadProgress',
				data: { downloaded: 512, total: 1024 }
			});
		});

		await installAppUpdate('0.1.1', (event) => events.push(event));

		expect(invoke).toHaveBeenCalledWith('install_app_update', {
			expectedVersion: '0.1.1',
			onEvent: expect.anything()
		});
		expect(events).toEqual([
			{ event: 'downloadProgress', data: { downloaded: 512, total: 1024 } }
		]);
	});
});
