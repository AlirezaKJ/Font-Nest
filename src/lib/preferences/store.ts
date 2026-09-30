import type { LoadedPreferences } from '$lib/bindings/LoadedPreferences';
import type { Preferences } from '$lib/bindings/Preferences';
import {
	loadPreferences as loadNativePreferences,
	savePreferences as saveNativePreferences
} from '$lib/tauri/commands';

/**
 * Where FontNest's settings live.
 *
 * In the desktop application they are a document the Rust side owns: versioned, written through a
 * temporary file, and recovered from when it comes back unreadable. Browser development has no
 * such side, so it keeps using `localStorage`, which is what the desktop app used before the store
 * existed and is exactly what that build wants: throwaway settings for a throwaway catalogue.
 *
 * The first desktop launch after the move adopts whatever the web view was already holding, so
 * nobody loses their theme, their specimen or their saved previews to an implementation detail.
 */

/** The key the web view used before the settings became FontNest's own document. */
export const LEGACY_STORAGE_KEY = 'fontnest.preferences.v1';

export const DEFAULT_PREFERENCES: Preferences = {
	theme: 'system',
	density: 'comfortable',
	focusOutlines: false,
	previewText: '',
	sidebarCollapsed: false,
	pinnedFamilyIds: [],
	session: null
};

export function isDesktop(): boolean {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Reads a stored record into settings, keeping only what still makes sense.
 *
 * The legacy record was written by an older build and lives in storage a person can edit, so
 * every field is checked on its own and a bad one falls back to its default rather than taking
 * the rest with it.
 */
export function parsePreferences(raw: unknown): Preferences {
	const stored = (typeof raw === 'object' && raw !== null ? raw : {}) as Record<string, unknown>;
	const preferences: Preferences = { ...DEFAULT_PREFERENCES };

	if (stored.theme === 'system' || stored.theme === 'light' || stored.theme === 'dark') {
		preferences.theme = stored.theme;
	}
	if (stored.density === 'comfortable' || stored.density === 'compact') {
		preferences.density = stored.density;
	}
	if (typeof stored.focusOutlines === 'boolean') preferences.focusOutlines = stored.focusOutlines;
	if (typeof stored.previewText === 'string') preferences.previewText = stored.previewText;
	if (typeof stored.sidebarCollapsed === 'boolean') {
		preferences.sidebarCollapsed = stored.sidebarCollapsed;
	}
	if (Array.isArray(stored.pinnedFamilyIds)) {
		preferences.pinnedFamilyIds = [
			...new Set(
				stored.pinnedFamilyIds.filter((value): value is string => typeof value === 'string')
			)
		];
	}
	if (typeof stored.session === 'object' && stored.session !== null) {
		preferences.session = stored.session as Record<string, unknown>;
	}

	return preferences;
}

/** Whatever the web view is still holding from before the move, if anything. */
function legacyPreferences(): Preferences | null {
	try {
		const stored = localStorage.getItem(LEGACY_STORAGE_KEY);
		return stored === null ? null : parsePreferences(JSON.parse(stored));
	} catch {
		return null;
	}
}

function browserSave(preferences: Preferences): void {
	try {
		localStorage.setItem(LEGACY_STORAGE_KEY, JSON.stringify(preferences));
	} catch {
		// Private browsing, a full quota, or storage turned off. Development settings are not
		// worth interrupting anyone over.
	}
}

/**
 * Whether a loaded record is still untouched defaults, which is how a first launch looks and is
 * the only time adopting the web view's old settings is the right thing to do.
 */
export function isUntouched(preferences: Preferences): boolean {
	return (
		preferences.theme === DEFAULT_PREFERENCES.theme &&
		preferences.density === DEFAULT_PREFERENCES.density &&
		preferences.focusOutlines === DEFAULT_PREFERENCES.focusOutlines &&
		preferences.previewText === DEFAULT_PREFERENCES.previewText &&
		preferences.sidebarCollapsed === DEFAULT_PREFERENCES.sidebarCollapsed &&
		preferences.pinnedFamilyIds.length === 0 &&
		preferences.session === null
	);
}

export type PreferencesLoad = LoadedPreferences & {
	/** True when this launch adopted settings the web view had been holding. */
	migrated: boolean;
};

export async function load(): Promise<PreferencesLoad> {
	if (!isDesktop()) {
		return {
			preferences: legacyPreferences() ?? { ...DEFAULT_PREFERENCES },
			recovery: null,
			migrated: false
		};
	}

	const loaded = await loadNativePreferences();

	// A store with nothing in it and a web view that still has the old record: this is the first
	// launch after the move, so adopt it and write it where it belongs now.
	if (loaded.recovery === null && isUntouched(loaded.preferences)) {
		const legacy = legacyPreferences();
		if (legacy && !isUntouched(legacy)) {
			await saveNativePreferences(legacy);
			forgetLegacyPreferences();
			return { preferences: legacy, recovery: null, migrated: true };
		}
	}

	forgetLegacyPreferences();
	return { ...loaded, migrated: false };
}

/** Drops the web view's copy, so two records can never drift apart. */
function forgetLegacyPreferences(): void {
	try {
		localStorage.removeItem(LEGACY_STORAGE_KEY);
	} catch {
		// Nothing to do: the copy that matters is the one FontNest owns.
	}
}

export async function save(preferences: Preferences): Promise<void> {
	if (!isDesktop()) {
		browserSave(preferences);
		return;
	}
	await saveNativePreferences(preferences);
}
