import { get } from 'svelte/store';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { checkForAppUpdate, installAppUpdate } from '$lib/tauri/commands';
import {
	appUpdater,
	checkForUpdates,
	installAvailableUpdate,
	progressPercent,
	resetAppUpdater
} from './app-updater';

vi.mock('$lib/tauri/commands', () => ({
	checkForAppUpdate: vi.fn(),
	installAppUpdate: vi.fn()
}));

describe('app updater state', () => {
	beforeEach(() => {
		resetAppUpdater();
		vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });
		vi.mocked(checkForAppUpdate).mockReset();
		vi.mocked(installAppUpdate).mockReset();
	});

	afterEach(() => {
		vi.unstubAllGlobals();
	});

	it('presents an update returned by the trusted Rust command', async () => {
		const update = {
			currentVersion: '0.1.0',
			version: '0.1.1',
			notes: 'Updater ready.',
			publishedAt: null
		};
		vi.mocked(checkForAppUpdate).mockResolvedValue(update);

		await expect(checkForUpdates()).resolves.toEqual(update);
		expect(get(appUpdater)).toMatchObject({ status: 'available', update });
	});

	it('tracks download progress and installation state', async () => {
		const update = {
			currentVersion: '0.1.0',
			version: '0.1.1',
			notes: '',
			publishedAt: null
		};
		vi.mocked(checkForAppUpdate).mockResolvedValue(update);
		vi.mocked(installAppUpdate).mockImplementation(async (_version, onEvent) => {
			onEvent({ event: 'downloadStarted', data: { total: 1000 } });
			onEvent({ event: 'downloadProgress', data: { downloaded: 500, total: 1000 } });
			onEvent({ event: 'installing' });
		});

		await checkForUpdates();
		await installAvailableUpdate();

		expect(installAppUpdate).toHaveBeenCalledWith('0.1.1', expect.any(Function));
		expect(get(appUpdater)).toMatchObject({
			status: 'installing',
			downloaded: 500,
			total: 1000
		});
	});

	it('calculates a bounded percentage only when total size is known', () => {
		expect(progressPercent(250, 1000)).toBe(25);
		expect(progressPercent(2000, 1000)).toBe(100);
		expect(progressPercent(10, null)).toBeNull();
	});
});
