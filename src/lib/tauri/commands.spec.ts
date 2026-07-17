import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { greet, scanInstalledFonts } from './commands';

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
