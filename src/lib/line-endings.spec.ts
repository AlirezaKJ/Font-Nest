import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { repoRoot } from '../../scripts/versions.mjs';

function repoFile(relative: string): string {
	return readFileSync(path.join(repoRoot, relative), 'utf8');
}

/**
 * One line per tracked file: how it is stored, how it sits in the working tree, and the attributes
 * that decided both. A CRLF checkout used to make `pnpm lint` fail on 91 files that had nothing
 * wrong with them, so the rule that prevents it is worth a test.
 */
function trackedFileEndings(): string[] {
	return execFileSync('git', ['ls-files', '--eol'], { cwd: repoRoot, encoding: 'utf8' })
		.split('\n')
		.filter((line) => line.trim().length > 0);
}

describe('line endings', () => {
	it('checks every text file out as LF whatever core.autocrlf says', () => {
		expect(repoFile('.gitattributes')).toMatch(/^\* text=auto eol=lf\r?$/m);
	});

	it('keeps Prettier and editors on the same ending', () => {
		expect(repoFile('prettier.config.js')).toMatch(/endOfLine: 'lf'/);
		expect(repoFile('.editorconfig')).toMatch(/^end_of_line = lf\r?$/m);
		// Not everyone installs the EditorConfig extension, and a new file in VS Code otherwise
		// takes the platform default, which is CRLF on Windows.
		const vscodeSettings = JSON.parse(repoFile('.vscode/settings.json')) as Record<
			string,
			unknown
		>;
		expect(vscodeSettings['files.eol']).toBe('\n');
	});

	it('holds no CRLF in the repository or the working tree', () => {
		const offenders = trackedFileEndings().filter((line) =>
			/\b[iw]\/(crlf|mixed)\b/.test(line)
		);

		expect(offenders).toEqual([]);
	});
});
