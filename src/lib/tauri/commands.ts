import { invoke } from '@tauri-apps/api/core';

import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
import type { Greeting } from '$lib/bindings/Greeting';

export function greet(name: string): Promise<Greeting> {
	return invoke<Greeting>('greet', { name });
}

export function scanInstalledFonts(): Promise<FontCatalogue> {
	return invoke<FontCatalogue>('scan_installed_fonts');
}
