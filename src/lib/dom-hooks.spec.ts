import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { repoRoot } from '../../scripts/versions.mjs';

/**
 * Attribute selectors are a contract between two files that the compiler cannot see.
 *
 * The keyboard shortcut that focuses the search box lives in the route and the box itself lives in
 * the title bar, joined only by the string `[data-font-search]`. A refactor renamed an identifier
 * inside that string, which broke the shortcut silently: nothing failed to compile, no test
 * noticed, and the key simply stopped working.
 */
function sources(directory = path.join(repoRoot, 'src')): { file: string; text: string }[] {
	return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
		const full = path.join(directory, entry.name);
		if (entry.isDirectory()) return sources(full);
		if (!/\.(svelte|ts)$/.test(entry.name) || entry.name.endsWith('.spec.ts')) return [];
		return [{ file: full, text: readFileSync(full, 'utf8') }];
	});
}

describe('selectors that join one file to another', () => {
	it('queries only data attributes that something actually renders', () => {
		const all = sources();
		const rendered = new Set<string>();
		for (const { text } of all) {
			for (const match of text.matchAll(/(?:^|[\s{])(data-[a-z0-9-]+)(?==|[\s>}])/gm)) {
				rendered.add(match[1]);
			}
		}

		const queried: { attribute: string; file: string }[] = [];
		for (const { file, text } of all) {
			for (const match of text.matchAll(
				/querySelector(?:All)?<[^>]*>?\(\s*'\[(data-[^\]'=]+)/g
			)) {
				queried.push({ attribute: match[1], file: path.relative(repoRoot, file) });
			}
		}

		expect(queried.length).toBeGreaterThan(0);
		const orphaned = queried.filter((entry) => !rendered.has(entry.attribute));
		expect(orphaned, `queried but never rendered: ${JSON.stringify(orphaned)}`).toEqual([]);
	});

	// The shortcut itself: pressing / anywhere outside a text field focuses the search box.
	it('still binds the key that focuses the search box', () => {
		const route = readFileSync(path.join(repoRoot, 'src/routes/+page.svelte'), 'utf8');
		expect(route).toContain("event.key === '/'");
		expect(route).toMatch(/event\.key === '\/'[\s\S]{0,200}focusSearch\(\)/);
	});
});
