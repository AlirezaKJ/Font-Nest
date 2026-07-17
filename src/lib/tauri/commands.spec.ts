import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
	getGoogleFontDetails,
	greet,
	installGoogleFont,
	listGoogleFonts,
	prepareGoogleFontPreview,
	scanInstalledFonts
} from './commands';

vi.mock('@tauri-apps/api/core', () => ({
	invoke: vi.fn()
}));

describe('greet', () => {
	beforeEach(() => {
		vi.mocked(invoke).mockReset();
	});

	it('invokes the typed Rust greeting command', async () => {
		const response = {
			appName: 'FontNest',
			message: 'Welcome to FontNest, Akari.',
			version: '0.1.0'
		};
		vi.mocked(invoke).mockResolvedValue(response);

		await expect(greet('Akari')).resolves.toEqual(response);
		expect(invoke).toHaveBeenCalledWith('greet', { name: 'Akari' });
	});
});

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

		await expect(
			listGoogleFonts({ query: 'serif', category: 'serif', offset: 0, limit: 60 })
		).resolves.toEqual(response);
		expect(invoke).toHaveBeenCalledWith('list_google_fonts', {
			request: { query: 'serif', category: 'serif', offset: 0, limit: 60 }
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
});
