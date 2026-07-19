import type { ValidatedLocalFont } from '$lib/bindings/ValidatedLocalFont';
import { validateFontFile } from '$lib/tauri/commands';

// SFNT containers the Rust validator can parse today. WOFF/WOFF2 are not decoded
// yet, so they are deliberately left out of the picker.
export const LOCAL_FONT_EXTENSIONS = ['ttf', 'otf', 'ttc', 'otc'] as const;

type FilePicker = () => Promise<string | null>;
type Validator = (path: string) => Promise<ValidatedLocalFont>;

// Preview faces registered on the document, keyed by their synthetic family name.
const loadedPreviews = new Map<string, FontFace>();

async function openFontDialog(): Promise<string | null> {
	const { open } = await import('@tauri-apps/plugin-dialog');
	const selection = await open({
		multiple: false,
		directory: false,
		title: 'Preview a local font',
		filters: [{ name: 'Desktop fonts', extensions: [...LOCAL_FONT_EXTENSIONS] }]
	});
	return typeof selection === 'string' ? selection : null;
}

/**
 * Loads already-validated preview bytes into the current web view under the font's
 * synthetic family name, so a duplicate installed family cannot shadow it. The bytes
 * come from the internal preview protocol keyed by the opaque handle, never a path.
 * Safe to call more than once; each family is registered a single time.
 */
export async function activateLocalFontPreview(validated: ValidatedLocalFont): Promise<void> {
	if (loadedPreviews.has(validated.previewFamily)) return;
	const face = new FontFace(validated.previewFamily, `url("${validated.previewUrl}")`);
	await face.load();
	document.fonts.add(face);
	loadedPreviews.set(validated.previewFamily, face);
}

/**
 * Opens the trusted file dialog, sends the chosen path across the Rust validation
 * boundary, and on success loads the validated bytes for preview. Returns the
 * validated summary, or null when the user dismisses the dialog. The path never
 * reaches the web view; only the returned handle and metadata do.
 */
export async function importLocalFontPreview(
	pick: FilePicker = openFontDialog,
	validate: Validator = validateFontFile
): Promise<ValidatedLocalFont | null> {
	const path = await pick();
	if (!path) return null;
	const validated = await validate(path);
	await activateLocalFontPreview(validated);
	return validated;
}

/** Releases every loaded local preview face from the document. */
export function clearLocalFontPreviews(): void {
	for (const face of loadedPreviews.values()) document.fonts.delete(face);
	loadedPreviews.clear();
}
