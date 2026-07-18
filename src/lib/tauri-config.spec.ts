import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

type TauriConfig = {
	app?: {
		windows?: Array<{
			label?: string;
			dragDropEnabled?: boolean;
		}>;
	};
};

describe('Tauri window configuration', () => {
	it('leaves drag and drop to HTML in the main Windows webview', () => {
		const configPath = new URL('../../src-tauri/tauri.conf.json', import.meta.url);
		const config = JSON.parse(readFileSync(configPath, 'utf8')) as TauriConfig;
		const mainWindow = config.app?.windows?.find((window) => window.label === 'main');

		expect(mainWindow?.dragDropEnabled).toBe(false);
	});
});
