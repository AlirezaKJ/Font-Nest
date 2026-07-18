import { Channel, invoke } from '@tauri-apps/api/core';

import type { AppUpdateEvent } from '$lib/bindings/AppUpdateEvent';
import type { AppUpdateInfo } from '$lib/bindings/AppUpdateInfo';
import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
import type { FontFaceInspection } from '$lib/bindings/FontFaceInspection';
import type { FontGlyphOutline } from '$lib/bindings/FontGlyphOutline';
import type { FontGlyphOutlineRequest } from '$lib/bindings/FontGlyphOutlineRequest';
import type { FontParserJsonExport } from '$lib/bindings/FontParserJsonExport';
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

export function inspectFontFace(faceId: string): Promise<FontFaceInspection> {
	return invoke<FontFaceInspection>('inspect_font_face', { faceId });
}

export function inspectFontGlyphOutline(
	request: FontGlyphOutlineRequest
): Promise<FontGlyphOutline> {
	return invoke<FontGlyphOutline>('inspect_font_glyph_outline', { request });
}

export function exportFontFaceParserJson(faceId: string): Promise<FontParserJsonExport> {
	return invoke<FontParserJsonExport>('export_font_face_parser_json', { faceId });
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

export function checkForAppUpdate(): Promise<AppUpdateInfo | null> {
	return invoke<AppUpdateInfo | null>('check_for_app_update');
}

export function installAppUpdate(
	expectedVersion: string,
	onEvent: (event: AppUpdateEvent) => void
): Promise<void> {
	const channel = new Channel<AppUpdateEvent>();
	channel.onmessage = onEvent;
	return invoke<void>('install_app_update', { expectedVersion, onEvent: channel });
}
