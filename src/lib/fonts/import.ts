import type { ImportCandidate } from '$lib/bindings/ImportCandidate';
import type { ImportOutcome } from '$lib/bindings/ImportOutcome';
import type { ImportPlan } from '$lib/bindings/ImportPlan';
import type { ImportVerdict } from '$lib/bindings/ImportVerdict';
import { importFontFiles, preflightFontImport } from '$lib/tauri/commands';

/**
 * Choosing font files to import, and saying plainly what FontNest found in them.
 *
 * The paths a picker returns are a question, not an instruction: every one of them is reviewed in
 * Rust, from the bytes, and reviewed again when an import runs. Nothing here decides whether a
 * file may be installed; it decides how to describe the answer.
 */

/** Containers the importer will open. Web fonts are not desktop fonts and are not offered. */
export const IMPORTABLE_EXTENSIONS = ['ttf', 'otf', 'ttc', 'otc'] as const;

type FilePicker = (options: { directory: boolean }) => Promise<string[]>;

async function openImportDialog({ directory }: { directory: boolean }): Promise<string[]> {
	const { open } = await import('@tauri-apps/plugin-dialog');
	const selection = await open({
		multiple: true,
		directory,
		title: directory ? 'Import a folder of fonts' : 'Import font files',
		filters: directory
			? undefined
			: [{ name: 'Desktop fonts', extensions: [...IMPORTABLE_EXTENSIONS] }]
	});
	if (selection === null) return [];
	return Array.isArray(selection) ? selection : [selection];
}

/** What a verdict means, in the second person, because it is the person's file being described. */
const VERDICTS: Record<ImportVerdict, { label: string; detail: string; tone: Tone }> = {
	installable: {
		label: 'Ready to install',
		detail: 'A font this computer can use, and one FontNest has not installed already.',
		tone: 'ready'
	},
	alreadyInstalled: {
		label: 'Already installed',
		detail: 'FontNest installed exactly these bytes before. Importing again would change nothing.',
		tone: 'settled'
	},
	repeatedSelection: {
		label: 'Chosen twice',
		detail: 'The same font is in this selection more than once. It will be installed once.',
		tone: 'settled'
	},
	previewOnly: {
		label: 'Preview only',
		detail: 'A real font, in a format Windows does not install from a file: collections and web fonts can be looked at but not installed.',
		tone: 'blocked'
	},
	unreadable: {
		label: 'Will not open',
		detail: 'Nothing here parses as a font, or its naming metadata is unusable.',
		tone: 'blocked'
	},
	tooLarge: {
		label: 'Too large',
		detail: 'Larger than FontNest will read. Desktop fonts are a fraction of this size.',
		tone: 'blocked'
	},
	systemOwned: {
		label: 'Belongs to Windows',
		detail: 'This font is already installed by the operating system, and is not FontNest’s to claim.',
		tone: 'blocked'
	},
	missing: {
		label: 'Could not be opened',
		detail: 'The file is no longer where it was chosen from, or cannot be read.',
		tone: 'blocked'
	}
};

export type Tone = 'ready' | 'settled' | 'blocked';

export function verdictLabel(verdict: ImportVerdict): string {
	return VERDICTS[verdict].label;
}

export function verdictDetail(verdict: ImportVerdict): string {
	return VERDICTS[verdict].detail;
}

export function verdictTone(verdict: ImportVerdict): Tone {
	return VERDICTS[verdict].tone;
}

/** What a font says about embedding, in words rather than in bit values. */
export function embeddingLabel(candidate: ImportCandidate): string | null {
	switch (candidate.licence?.embedding) {
		case 'installable':
			return 'Embedding unrestricted';
		case 'restricted':
			return 'Embedding not permitted';
		case 'previewAndPrint':
			return 'Embedding for preview and print';
		case 'editable':
			return 'Embedding editable';
		default:
			return null;
	}
}

export function importableCount(plan: ImportPlan): number {
	return plan.candidates.filter((candidate) => candidate.verdict === 'installable').length;
}

/** A short line for the top of the review, so the shape of it is clear before reading the list. */
export function planSummary(plan: ImportPlan): string {
	const total = plan.candidates.length;
	if (total === 0) return 'Nothing to import.';

	const ready = importableCount(plan);
	const files = total === 1 ? '1 file' : `${total} files`;
	if (ready === 0) return `${files}, none of which can be installed.`;
	if (ready === total) return `${files}, all ready to install.`;
	return `${files}, ${ready} ready to install.`;
}

/** A line for what actually happened, once an import has run. */
export function outcomeSummary(outcomes: ImportOutcome[]): string {
	const installed = outcomes.filter((outcome) => outcome.installed).length;
	const failed = outcomes.filter((outcome) => outcome.failure !== null).length;

	if (installed === 0 && failed === 0) return 'Nothing was installed.';
	const fonts = installed === 1 ? '1 font' : `${installed} fonts`;
	if (failed === 0) return `${fonts} installed.`;
	return `${fonts} installed, ${failed === 1 ? '1 file' : `${failed} files`} could not be.`;
}

/**
 * Asks for font files and reviews them. Returns null when the picker was dismissed, so the caller
 * can tell "changed their mind" apart from "chose files that cannot be installed".
 */
export async function reviewChosenFonts(
	directory: boolean,
	pick: FilePicker = openImportDialog,
	preflight: (paths: string[]) => Promise<ImportPlan> = preflightFontImport
): Promise<ImportPlan | null> {
	const paths = await pick({ directory });
	if (paths.length === 0) return null;
	return preflight(paths);
}

/** Imports the files a review found installable, and nothing else. */
export async function importReviewed(
	plan: ImportPlan,
	run: (paths: string[]) => Promise<ImportOutcome[]> = importFontFiles
): Promise<ImportOutcome[]> {
	const paths = plan.candidates
		.filter((candidate) => candidate.verdict === 'installable')
		.map((candidate) => candidate.sourcePath);
	if (paths.length === 0) return [];
	return run(paths);
}
