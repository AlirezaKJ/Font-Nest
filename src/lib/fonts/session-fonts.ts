import type { GoogleFontArtifactSummary } from '$lib/bindings/GoogleFontArtifactSummary';
import type { GoogleFontPreview } from '$lib/bindings/GoogleFontPreview';
import { prepareGoogleFontPreview } from '$lib/tauri/commands';

type PreviewLoader = (artifactId: string) => Promise<GoogleFontPreview>;

/** Joins a family and an artifact into one key, using a character neither can hold. */
const SEPARATOR = String.fromCharCode(0);

const sessionFaces = new Map<string, FontFace>();
const pendingFaces = new Map<string, Promise<void>>();

/** One session face is a family and an artifact together, joined by a character neither can hold. */
function faceKey(familyName: string, artifactId: string): string {
	return familyName + SEPARATOR + artifactId;
}

export function fontFaceDescriptors(styleName: string): FontFaceDescriptors {
	const normalized = styleName.replace(/[-_]/g, ' ').toLocaleLowerCase();
	const style = normalized.includes('italic')
		? 'italic'
		: normalized.includes('oblique')
			? 'oblique'
			: 'normal';

	let weight = '400';
	if (normalized.includes('variable') && /\bwght\b/.test(normalized)) {
		weight = '100 900';
	} else if (/\b(extra|ultra)\s*light\b/.test(normalized)) {
		weight = '200';
	} else if (/\b(thin|hairline)\b/.test(normalized)) {
		weight = '100';
	} else if (/\b(light|book)\b/.test(normalized)) {
		weight = '300';
	} else if (/\bmedium\b/.test(normalized)) {
		weight = '500';
	} else if (/\b(semi|demi)\s*bold\b/.test(normalized)) {
		weight = '600';
	} else if (/\b(extra|ultra)\s*bold\b/.test(normalized)) {
		weight = '800';
	} else if (/\bblack\b|\bheavy\b/.test(normalized)) {
		weight = '900';
	} else if (/\bbold\b/.test(normalized)) {
		weight = '700';
	}

	return { style, weight };
}

export async function activateInstalledGoogleFont(
	familyName: string,
	artifacts: GoogleFontArtifactSummary[],
	loadPreview: PreviewLoader = prepareGoogleFontPreview
): Promise<void> {
	await Promise.all(
		artifacts.map(async (artifact) => {
			const key = `${familyName}\u0000${artifact.id}`;
			if (sessionFaces.has(key)) return;

			let pending = pendingFaces.get(key);
			if (!pending) {
				pending = (async () => {
					const preview = await loadPreview(artifact.id);
					const face = new FontFace(
						familyName,
						`url(${preview.previewUrl})`,
						fontFaceDescriptors(artifact.style)
					);
					await face.load();
					document.fonts.add(face);
					sessionFaces.set(key, face);
				})().finally(() => pendingFaces.delete(key));
				pendingFaces.set(key, pending);
			}

			await pending;
		})
	);
}

/**
 * Stops serving faces this session loaded for a family, so a font that has just been removed
 * stops rendering instead of lingering until the next launch.
 */
export function deactivateGoogleFontArtifacts(familyName: string, artifactIds: string[]): void {
	for (const artifactId of artifactIds) {
		const key = faceKey(familyName, artifactId);
		const face = sessionFaces.get(key);
		if (!face) continue;
		document.fonts.delete(face);
		sessionFaces.delete(key);
	}
}

export function clearSessionFontFaces(): void {
	for (const face of sessionFaces.values()) document.fonts.delete(face);
	sessionFaces.clear();
}
