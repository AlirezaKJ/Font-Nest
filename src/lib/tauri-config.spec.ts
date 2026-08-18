import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

type TauriConfig = {
	app?: {
		security?: {
			csp?: string;
		};
		windows?: Array<{
			label?: string;
			dragDropEnabled?: boolean;
			minWidth?: number;
		}>;
	};
	bundle?: {
		createUpdaterArtifacts?: boolean;
	};
	plugins?: {
		updater?: {
			pubkey?: string;
			endpoints?: string[];
			windows?: {
				installMode?: string;
			};
		};
	};
};

describe('Tauri window configuration', () => {
	it('leaves drag and drop to HTML in the main Windows webview', () => {
		const configPath = new URL('../../src-tauri/tauri.conf.json', import.meta.url);
		const config = JSON.parse(readFileSync(configPath, 'utf8')) as TauriConfig;
		const mainWindow = config.app?.windows?.find((window) => window.label === 'main');

		expect(mainWindow?.dragDropEnabled).toBe(false);
	});

	it('builds signed updater artifacts from the trusted GitHub release feed', () => {
		const configPath = new URL('../../src-tauri/tauri.conf.json', import.meta.url);
		const config = JSON.parse(readFileSync(configPath, 'utf8')) as TauriConfig;
		const updater = config.plugins?.updater;

		expect(config.bundle?.createUpdaterArtifacts).toBe(true);
		expect(updater?.pubkey).toBe(
			'dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDUxMkI4NDFCQUMzNEIwMEEKUldRS3NEU3NHNFFyVVkyR1kyQW8vQ21vVjNIRVYzSmlQYlRzMjFmamN2R1dCbThyN0oybTlyY2wK'
		);
		expect(updater?.endpoints).toEqual([
			'https://github.com/AlirezaKJ/Font-Nest/releases/latest/download/latest.json'
		]);
		expect(updater?.windows?.installMode).toBe('passive');
	});

	// Every font in the app is either bundled with it or served by handle through the
	// preview protocol. Nothing arrives as a data URL, and the policy is what makes that
	// a rule rather than a habit.
	it('only lets fonts load from the app and the preview protocol', () => {
		const configPath = new URL('../../src-tauri/tauri.conf.json', import.meta.url);
		const config = JSON.parse(readFileSync(configPath, 'utf8')) as TauriConfig;
		const fontSrc = /font-src ([^;]*)/.exec(config.app?.security?.csp ?? '')?.[1]?.trim();

		expect(fontSrc).toBeDefined();
		expect(fontSrc?.split(/\s+/)).toContain('http://fontnest-preview.localhost');
		expect(fontSrc).not.toContain('data:');
	});

	// A compact breakpoint the window can never reach is dead CSS that nobody can
	// review. Keep the floor at or below the narrowest one we author.
	it('lets the window shrink to the narrowest authored breakpoint', () => {
		const configPath = new URL('../../src-tauri/tauri.conf.json', import.meta.url);
		const config = JSON.parse(readFileSync(configPath, 'utf8')) as TauriConfig;
		const mainWindow = config.app?.windows?.find((window) => window.label === 'main');
		const narrowest = Math.min(...collectMaxWidthBreakpoints());

		expect(Number.isFinite(narrowest)).toBe(true);
		expect(mainWindow?.minWidth).toBeLessThanOrEqual(narrowest);
	});
});

function collectMaxWidthBreakpoints(): number[] {
	const root = fileURLToPath(new URL('..', import.meta.url));
	const breakpoints: number[] = [];

	for (const entry of readdirSync(root, { recursive: true, withFileTypes: true })) {
		if (!entry.isFile() || !entry.name.endsWith('.svelte')) continue;
		const source = readFileSync(`${entry.parentPath}/${entry.name}`, 'utf8');
		for (const match of source.matchAll(/@media[^{]*\(\s*max-width:\s*(\d+)px/g)) {
			breakpoints.push(Number(match[1]));
		}
	}

	return breakpoints;
}
