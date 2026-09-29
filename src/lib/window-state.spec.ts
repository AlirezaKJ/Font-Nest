import { readFileSync } from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { repoRoot } from '../../scripts/versions.mjs';

/**
 * The window's smallest size is written in two places: `tauri.conf.json`, which the window
 * manager enforces, and `window_state.rs`, which will not restore a saved rectangle below it. If
 * those two drift apart, a restored window can end up smaller than the interface can lay out in.
 */
describe('window state', () => {
	it('clamps restored windows to the same minimum the window itself has', () => {
		const config = JSON.parse(
			readFileSync(path.join(repoRoot, 'src-tauri/tauri.conf.json'), 'utf8')
		) as {
			app?: { windows?: Array<{ label?: string; minWidth?: number; minHeight?: number }> };
		};
		const main = config.app?.windows?.find((window) => window.label === 'main');

		const source = readFileSync(path.join(repoRoot, 'src-tauri/src/window_state.rs'), 'utf8');
		const declared = /MINIMUM_LOGICAL_SIZE: \(u32, u32\) = \((\d+), (\d+)\);/.exec(source);

		expect(declared).not.toBeNull();
		expect(Number(declared?.[1])).toBe(main?.minWidth);
		expect(Number(declared?.[2])).toBe(main?.minHeight);
	});

	// The window opens hidden and the frontend reveals it after the first themed frame. Restoring
	// the rectangle has to happen inside that window, or moving it is something you watch happen.
	it('restores the rectangle before the window is ever shown', () => {
		const source = readFileSync(path.join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
		const restoreAt = source.indexOf('window_state::restore');
		const revealAt = source.indexOf('window.show()');

		expect(restoreAt).toBeGreaterThan(-1);
		expect(revealAt).toBeGreaterThan(-1);
		expect(restoreAt).toBeLessThan(revealAt);
	});
});
