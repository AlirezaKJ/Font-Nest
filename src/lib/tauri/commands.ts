import { invoke } from '@tauri-apps/api/core';

import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
import type { Greeting } from '$lib/bindings/Greeting';
import type { GoogleFontFamilyDetails } from '$lib/bindings/GoogleFontFamilyDetails';
import type { GoogleFontInstallResult } from '$lib/bindings/GoogleFontInstallResult';
import type { GoogleFontPage } from '$lib/bindings/GoogleFontPage';
import type { GoogleFontPageRequest } from '$lib/bindings/GoogleFontPageRequest';
import type { GoogleFontPreview } from '$lib/bindings/GoogleFontPreview';

export function greet(name: string): Promise<Greeting> {
	return invoke<Greeting>('greet', { name });
}

export function scanInstalledFonts(): Promise<FontCatalogue> {
	return invoke<FontCatalogue>('scan_installed_fonts');
}

export function listGoogleFonts(request: GoogleFontPageRequest): Promise<GoogleFontPage> {
	return invoke<GoogleFontPage>('list_google_fonts', { request });
}

export function getGoogleFontDetails(familyId: string): Promise<GoogleFontFamilyDetails> {
	return invoke<GoogleFontFamilyDetails>('get_google_font_details', { familyId });
}

export function prepareGoogleFontPreview(artifactId: string): Promise<GoogleFontPreview> {
	return invoke<GoogleFontPreview>('prepare_google_font_preview', { artifactId });
}

export function installGoogleFont(
	familyId: string,
	artifactIds: string[]
): Promise<GoogleFontInstallResult> {
	return invoke<GoogleFontInstallResult>('install_google_font', {
		request: { familyId, artifactIds }
	});
}
