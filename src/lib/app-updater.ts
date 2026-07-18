import { get, writable } from 'svelte/store';

import type { AppUpdateEvent } from '$lib/bindings/AppUpdateEvent';
import type { AppUpdateInfo } from '$lib/bindings/AppUpdateInfo';
import { checkForAppUpdate, installAppUpdate } from '$lib/tauri/commands';

export type AppUpdaterStatus =
	| 'idle'
	| 'checking'
	| 'current'
	| 'available'
	| 'downloading'
	| 'installing'
	| 'error'
	| 'unsupported';

export type AppUpdaterState = {
	status: AppUpdaterStatus;
	update: AppUpdateInfo | null;
	error: string;
	downloaded: number;
	total: number | null;
};

const initialState: AppUpdaterState = {
	status: 'idle',
	update: null,
	error: '',
	downloaded: 0,
	total: null
};

export const appUpdater = writable<AppUpdaterState>(initialState);

export function resetAppUpdater() {
	appUpdater.set(initialState);
}

export async function checkForUpdates(): Promise<AppUpdateInfo | null> {
	const current = get(appUpdater);
	if (current.status === 'downloading' || current.status === 'installing') {
		return current.update;
	}

	if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
		appUpdater.set({ ...initialState, status: 'unsupported' });
		return null;
	}

	appUpdater.update((state) => ({ ...state, status: 'checking', error: '' }));
	try {
		const update = await checkForAppUpdate();
		appUpdater.set({
			...initialState,
			status: update ? 'available' : 'current',
			update
		});
		return update;
	} catch (error) {
		appUpdater.update((state) => ({
			...state,
			status: 'error',
			error: commandErrorMessage(error)
		}));
		return null;
	}
}

export async function installAvailableUpdate(): Promise<void> {
	const { update } = get(appUpdater);
	if (!update) return;

	appUpdater.update((state) => ({
		...state,
		status: 'downloading',
		error: '',
		downloaded: 0,
		total: null
	}));

	try {
		await installAppUpdate(update.version, handleProgressEvent);
		appUpdater.update((state) => ({ ...state, status: 'installing' }));
	} catch (error) {
		appUpdater.update((state) => ({
			...state,
			status: 'error',
			error: commandErrorMessage(error)
		}));
	}
}

export function progressPercent(downloaded: number, total: number | null): number | null {
	if (!total || total <= 0) return null;
	return Math.min(100, Math.max(0, Math.round((downloaded / total) * 100)));
}

function handleProgressEvent(event: AppUpdateEvent) {
	if (event.event === 'downloadStarted') {
		appUpdater.update((state) => ({
			...state,
			status: 'downloading',
			downloaded: 0,
			total: event.data.total
		}));
	} else if (event.event === 'downloadProgress') {
		appUpdater.update((state) => ({
			...state,
			status: 'downloading',
			downloaded: event.data.downloaded,
			total: event.data.total
		}));
	} else {
		appUpdater.update((state) => ({ ...state, status: 'installing' }));
	}
}

function commandErrorMessage(error: unknown): string {
	if (typeof error === 'object' && error && 'message' in error) {
		return String(error.message);
	}
	return 'FontNest could not complete the update. Check your connection and try again.';
}
