/** Writes text to the system clipboard. Returns false when the web view refuses. */
export async function writeClipboardText(value: string): Promise<boolean> {
	if (!value) return false;
	try {
		await navigator.clipboard.writeText(value);
		return true;
	} catch {
		return false;
	}
}

/**
 * Reads the system clipboard. Chromium blocks synchronous paste from script, so this is
 * the only path available to the Paste action and it can legitimately be refused.
 */
export async function readClipboardText(): Promise<string | null> {
	try {
		return await navigator.clipboard.readText();
	} catch {
		return null;
	}
}
