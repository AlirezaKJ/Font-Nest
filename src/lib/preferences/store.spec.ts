import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { Preferences } from '$lib/bindings/Preferences';

const loadNative = vi.hoisted(() => vi.fn());
const saveNative = vi.hoisted(() => vi.fn());

vi.mock('$lib/tauri/commands', () => ({
	loadPreferences: loadNative,
	savePreferences: saveNative
}));

const { DEFAULT_PREFERENCES, LEGACY_STORAGE_KEY, isUntouched, load, parsePreferences, save } =
	await import('./store');

/** A minimal stand-in for the web view's storage, which the node test environment has none of. */
function fakeStorage(): Storage {
	const entries = new Map<string, string>();
	return {
		get length() {
			return entries.size;
		},
		clear: () => entries.clear(),
		getItem: (key: string) => entries.get(key) ?? null,
		key: (index: number) => [...entries.keys()][index] ?? null,
		removeItem: (key: string) => void entries.delete(key),
		setItem: (key: string, value: string) => void entries.set(key, value)
	};
}

function settings(overrides: Partial<Preferences> = {}): Preferences {
	return { ...DEFAULT_PREFERENCES, theme: 'dark', previewText: 'Hamburgefonstiv', ...overrides };
}

beforeEach(() => {
	loadNative.mockReset();
	saveNative.mockReset().mockResolvedValue(undefined);
	vi.stubGlobal('localStorage', fakeStorage());
	vi.stubGlobal('window', { localStorage } as unknown as Window);
});

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('parsePreferences', () => {
	it('keeps a record it recognizes', () => {
		expect(parsePreferences(settings({ pinnedFamilyIds: ['family:01'] }))).toEqual(
			settings({ pinnedFamilyIds: ['family:01'] })
		);
	});

	it('falls back per field rather than losing the record', () => {
		const parsed = parsePreferences({
			theme: 'aubergine',
			density: 'compact',
			focusOutlines: 'yes',
			previewText: 'Quousque tandem'
		});

		expect(parsed.theme).toBe('system');
		expect(parsed.focusOutlines).toBe(false);
		expect(parsed.density).toBe('compact');
		expect(parsed.previewText).toBe('Quousque tandem');
	});

	it('reads nothing out of nothing', () => {
		expect(parsePreferences(null)).toEqual(DEFAULT_PREFERENCES);
		expect(parsePreferences('dark')).toEqual(DEFAULT_PREFERENCES);
	});

	it('drops repeated saved previews', () => {
		const parsed = parsePreferences({ pinnedFamilyIds: ['family:01', 'family:01', 7] });

		expect(parsed.pinnedFamilyIds).toEqual(['family:01']);
	});
});

describe('isUntouched', () => {
	it('recognizes a store nobody has written to yet', () => {
		expect(isUntouched(DEFAULT_PREFERENCES)).toBe(true);
		expect(isUntouched(settings())).toBe(false);
		expect(isUntouched({ ...DEFAULT_PREFERENCES, session: {} })).toBe(false);
	});
});

describe('browser development', () => {
	it('keeps settings in web view storage when there is no desktop side', async () => {
		await save(settings());

		expect(saveNative).not.toHaveBeenCalled();
		expect((await load()).preferences).toEqual(settings());
	});
});

describe('the desktop store', () => {
	beforeEach(() => {
		vi.stubGlobal('window', { __TAURI_INTERNALS__: {}, localStorage } as unknown as Window);
	});

	it('reads the settings FontNest owns', async () => {
		loadNative.mockResolvedValue({ preferences: settings(), recovery: null });

		const loaded = await load();

		expect(loaded.preferences).toEqual(settings());
		expect(loaded.migrated).toBe(false);
	});

	// The first launch after the move: settings were in the web view, and they should not be lost
	// to an implementation detail.
	it('adopts what the web view was holding, once', async () => {
		localStorage.setItem(LEGACY_STORAGE_KEY, JSON.stringify(settings()));
		loadNative.mockResolvedValue({ preferences: DEFAULT_PREFERENCES, recovery: null });

		const loaded = await load();

		expect(loaded.migrated).toBe(true);
		expect(loaded.preferences).toEqual(settings());
		expect(saveNative).toHaveBeenCalledWith(settings());
		expect(localStorage.getItem(LEGACY_STORAGE_KEY)).toBeNull();
	});

	it('never lets the old copy overwrite settings the store already has', async () => {
		localStorage.setItem(LEGACY_STORAGE_KEY, JSON.stringify(settings({ theme: 'light' })));
		loadNative.mockResolvedValue({ preferences: settings(), recovery: null });

		const loaded = await load();

		expect(loaded.preferences.theme).toBe('dark');
		expect(loaded.migrated).toBe(false);
		expect(saveNative).not.toHaveBeenCalled();
		expect(localStorage.getItem(LEGACY_STORAGE_KEY)).toBeNull();
	});

	// Recovered settings are defaults, which looks exactly like a fresh store. Adopting the old
	// copy then would quietly undo the recovery.
	it('does not adopt the old copy over a recovery', async () => {
		localStorage.setItem(LEGACY_STORAGE_KEY, JSON.stringify(settings()));
		loadNative.mockResolvedValue({ preferences: DEFAULT_PREFERENCES, recovery: 'unreadable' });

		const loaded = await load();

		expect(loaded.preferences).toEqual(DEFAULT_PREFERENCES);
		expect(loaded.recovery).toBe('unreadable');
		expect(saveNative).not.toHaveBeenCalled();
	});

	it('writes through to the desktop side', async () => {
		await save(settings());

		expect(saveNative).toHaveBeenCalledWith(settings());
	});
});
