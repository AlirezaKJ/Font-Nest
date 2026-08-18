import { Channel, invoke } from '@tauri-apps/api/core';

import type { AppUpdateEvent } from '$lib/bindings/AppUpdateEvent';
import type { AppUpdateInfo } from '$lib/bindings/AppUpdateInfo';
import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
import type { FontFaceInspection } from '$lib/bindings/FontFaceInspection';
import type { FontGlyphOutline } from '$lib/bindings/FontGlyphOutline';
import type { FontGlyphOutlineRequest } from '$lib/bindings/FontGlyphOutlineRequest';
import type { FontParserJsonEvent } from '$lib/bindings/FontParserJsonEvent';
import type { FontParserJsonRequest } from '$lib/bindings/FontParserJsonRequest';
import type { GoogleFontFamilyDetails } from '$lib/bindings/GoogleFontFamilyDetails';
import type { GoogleFontInstallResult } from '$lib/bindings/GoogleFontInstallResult';
import type { GoogleFontPage } from '$lib/bindings/GoogleFontPage';
import type { GoogleFontPageRequest } from '$lib/bindings/GoogleFontPageRequest';
import type { GoogleFontPreview } from '$lib/bindings/GoogleFontPreview';
import type { ManagedStorageStatus } from '$lib/bindings/ManagedStorageStatus';
import type { ValidatedLocalFont } from '$lib/bindings/ValidatedLocalFont';

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

/**
 * Streams a face's parser snapshot. It arrives as ordered chunks rather than one string,
 * so a large font never becomes a single oversized payload.
 */
export function exportFontFaceParserJson(
	request: FontParserJsonRequest,
	onEvent: (event: FontParserJsonEvent) => void
): Promise<void> {
	const channel = new Channel<FontParserJsonEvent>();
	channel.onmessage = onEvent;
	return invoke<void>('export_font_face_parser_json', { request, onEvent: channel });
}

/** Abandons a running export. Unknown IDs are ignored. */
export function cancelFontFaceParserExport(exportId: string): Promise<void> {
	return invoke<void>('cancel_font_face_parser_export', { exportId });
}

export function fontFaceFilePath(faceId: string): Promise<string> {
	return invoke<string>('font_face_file_path', { faceId });
}

/** Resolves to false when only the containing folder could be opened. */
export function revealFontFaceFile(faceId: string): Promise<boolean> {
	return invoke<boolean>('reveal_font_face_file', { faceId });
}

export function validateFontFile(path: string): Promise<ValidatedLocalFont> {
	return invoke<ValidatedLocalFont>('validate_font_file', { path });
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

/** Whether this session may install, update, or remove managed fonts, and why not when it may not. */
export function managedStorageStatus(): Promise<ManagedStorageStatus> {
	return invoke<ManagedStorageStatus>('managed_storage_status');
}

export function installGoogleFont(
	familyId: string,
	artifactIds: string[]
): Promise<GoogleFontInstallResult> {
	return invoke<GoogleFontInstallResult>('install_google_font', {
		request: { familyId, artifactIds }
	});
}

export function fetchRemoteChangelog(): Promise<string> {
	return invoke<string>('fetch_remote_changelog');
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
