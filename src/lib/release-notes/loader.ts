import { writable } from 'svelte/store';

import bundledChangelog from '../../../CHANGELOG.md?raw';
import { fetchRemoteChangelog } from '$lib/tauri/commands';

import { compareVersions, parseChangelog, type Changelog } from './changelog';

/** Raw CHANGELOG.md on the default branch — the live feed for versions newer than installed. */
const REMOTE_CHANGELOG_URL =
	'https://raw.githubusercontent.com/AlirezaKJ/Font-Nest/main/CHANGELOG.md';

const LAST_SEEN_KEY = 'fontnest.releaseNotes.lastSeenVersion.v1';

export type ReleaseNotesSource = 'remote' | 'bundled';
export type ReleaseNotesStatus = 'idle' | 'loading' | 'ready' | 'error';

export type ReleaseNotesState = {
	status: ReleaseNotesStatus;
	changelog: Changelog | null;
	currentVersion: string | null;
	source: ReleaseNotesSource;
	error: string;
};

const currentVersion = typeof __APP_VERSION__ === 'string' ? __APP_VERSION__ : null;

const initialState: ReleaseNotesState = {
	status: 'idle',
	changelog: null,
	currentVersion,
	source: 'bundled',
	error: ''
};

export const releaseNotes = writable<ReleaseNotesState>(initialState);

function isTauri(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function fetchRemote(): Promise<string | null> {
	try {
		if (isTauri()) {
			// The webview cannot reach GitHub directly under the app CSP, so the Rust side fetches.
			return await fetchRemoteChangelog();
		}
		if (typeof fetch === 'undefined') return null;
		const response = await fetch(REMOTE_CHANGELOG_URL, {
			headers: { Accept: 'text/plain' },
			cache: 'no-cache'
		});
		return response.ok ? await response.text() : null;
	} catch {
		return null;
	}
}

/**
 * Loads release notes, preferring the freshest copy from GitHub and always falling back to the
 * copy bundled at build time so the view works offline and on first launch.
 */
export async function loadReleaseNotes(force = false): Promise<void> {
	let alreadyReady = false;
	releaseNotes.update((state) => {
		alreadyReady = state.status === 'ready';
		return state.status === 'loading' ? state : { ...state, status: 'loading', error: '' };
	});
	if (alreadyReady && !force) {
		releaseNotes.update((state) => ({ ...state, status: 'ready' }));
		return;
	}

	const bundled = parseChangelog(bundledChangelog);

	const remoteText = await fetchRemote();
	const remote = remoteText ? parseChangelog(remoteText) : null;
	const useRemote = Boolean(remote && remote.entries.length >= bundled.entries.length);

	const changelog = useRemote && remote ? remote : bundled;
	if (changelog.entries.length === 0) {
		releaseNotes.set({
			...initialState,
			status: 'error',
			error: 'FontNest could not read its release notes.'
		});
		return;
	}

	releaseNotes.set({
		status: 'ready',
		changelog,
		currentVersion,
		source: useRemote ? 'remote' : 'bundled',
		error: ''
	});
}

/** The application version whose notes the user last acknowledged, if any. */
export function lastSeenVersion(): string | null {
	try {
		return localStorage.getItem(LAST_SEEN_KEY);
	} catch {
		return null;
	}
}

/** Records the running version so the "what's new" prompt does not reappear for it. */
export function markCurrentVersionSeen(): void {
	if (!currentVersion) return;
	try {
		localStorage.setItem(LAST_SEEN_KEY, currentVersion);
	} catch {
		// Ignore storage failures; the prompt simply reappears next launch.
	}
}

/**
 * Whether the running version is newer than the last one the user acknowledged. Returns `false`
 * on a fresh install (nothing acknowledged yet) so first-run does not show an update prompt; the
 * current version is recorded silently instead.
 */
export function hasUnseenRelease(): boolean {
	if (!currentVersion) return false;
	const seen = lastSeenVersion();
	if (seen === null) {
		markCurrentVersionSeen();
		return false;
	}
	return compareVersions(currentVersion, seen) > 0;
}

export { currentVersion };
