import type { ValidatedLocalFont } from '$lib/bindings/ValidatedLocalFont';
import { validateFontFile } from '$lib/tauri/commands';

// SFNT containers the Rust validator can parse today. WOFF/WOFF2 are not decoded
// yet, so they are deliberately left out of the picker.
export const LOCAL_FONT_EXTENSIONS = ['ttf', 'otf', 'ttc', 'otc'] as const;

type FilePicker = () => Promise<string | null>;
type Validator = (path: string) => Promise<ValidatedLocalFont>;

// Preview faces registered on the document, keyed by their synthetic family name.
// The same family can be requested by more than one view at a time (a saved preview
// and the file dialog, say), so each entry counts its holders and only leaves the
// document once the last one lets go.
type LoadedPreview = { face: FontFace; holders: number };

const loadedPreviews = new Map<string, LoadedPreview>();
const pendingPreviews = new Map<string, Promise<void>>();

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
 * Safe to call more than once; each family is registered a single time and the extra
 * calls only take a hold on it. Every successful call must be paired with a
 * `releaseLocalFontPreview`, or the face stays on the document for the session.
 */
export async function activateLocalFontPreview(validated: ValidatedLocalFont): Promise<void> {
	const family = validated.previewFamily;
	const loaded = loadedPreviews.get(family);
	if (loaded) {
		loaded.holders += 1;
		return;
	}

	// Two views can ask for the same family before the first load settles; without this
	// they would each build a FontFace and only one of them would ever be released.
	const pending = pendingPreviews.get(family);
	if (pending) {
		await pending;
		const settled = loadedPreviews.get(family);
		if (settled) settled.holders += 1;
		return;
	}

	const load = (async () => {
		const face = new FontFace(family, `url("${validated.previewUrl}")`);
		await face.load();
		document.fonts.add(face);
		loadedPreviews.set(family, { face, holders: 1 });
	})().finally(() => pendingPreviews.delete(family));

	pendingPreviews.set(family, load);
	await load;
}

/**
 * Drops one hold on a preview family. The face leaves the document when the last
 * holder releases it, so switching or closing a preview does not leave the web view
 * carrying every font the user looked at this session.
 */
export function releaseLocalFontPreview(previewFamily: string | null | undefined): void {
	if (!previewFamily) return;
	const loaded = loadedPreviews.get(previewFamily);
	if (!loaded) return;

	loaded.holders -= 1;
	if (loaded.holders > 0) return;

	document.fonts.delete(loaded.face);
	loadedPreviews.delete(previewFamily);
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

/** Releases every loaded local preview face from the document, holders and all. */
export function clearLocalFontPreviews(): void {
	for (const loaded of loadedPreviews.values()) document.fonts.delete(loaded.face);
	loadedPreviews.clear();
	pendingPreviews.clear();
}
