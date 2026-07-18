import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

type TauriConfig = {
	app?: {
		windows?: Array<{
			label?: string;
			dragDropEnabled?: boolean;
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
});
