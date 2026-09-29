import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

import { describe, expect, it } from 'vitest';

import { repoRoot } from '../../scripts/versions.mjs';

const fontsCss = readFileSync(path.join(repoRoot, 'src/routes/fonts.css'), 'utf8');
const layoutCss = readFileSync(path.join(repoRoot, 'src/routes/layout.css'), 'utf8');

type FontFace = {
	block: string;
	family: string;
	style: string;
	weight: string;
	file: string;
};

function declaredFaces(): FontFace[] {
	return [...fontsCss.matchAll(/@font-face\s*\{([^}]*)\}/g)].map(([, block]) => ({
		block,
		family: /font-family:\s*([^;]+);/.exec(block)?.[1]?.trim().replace(/'/g, '') ?? '',
		style: /font-style:\s*([^;]+);/.exec(block)?.[1]?.trim() ?? '',
		weight: /font-weight:\s*([^;]+);/.exec(block)?.[1]?.trim() ?? '',
		file: /url\('([^']+)'\)/.exec(block)?.[1] ?? ''
	}));
}

describe('bundled interface typefaces', () => {
	// The design contract is explicit that FontNest renders its own frame without a network
	// connection, so a face named in the stack but missing from the repository is a bug.
	it('ships a file for every face it declares', () => {
		const faces = declaredFaces();

		expect(faces.length).toBeGreaterThan(0);
		for (const face of faces) {
			expect(face.file).toMatch(/^\/fonts\/[\w.-]+\.woff2$/);
			expect(existsSync(path.join(repoRoot, 'static', face.file))).toBe(true);
		}
	});

	it('serves Geist as one variable face in both postures', () => {
		const geist = declaredFaces().filter((face) => face.family === 'Geist');
		const postures = new Set(geist.map((face) => face.style));

		expect(geist.length).toBeGreaterThan(0);
		expect([...postures].sort()).toEqual(['italic', 'normal']);
		// The interface asks for 450, 550, 650 and 750, which only a weight axis can answer.
		for (const face of geist) {
			expect(face.weight).toBe('100 900');
		}
	});

	// Instrument Serif is display only, and the contract allows it in Regular alone.
	it('keeps Instrument Serif to upright Regular', () => {
		const serif = declaredFaces().filter((face) => face.family === 'Instrument Serif');

		expect(serif.length).toBeGreaterThan(0);
		for (const face of serif) {
			expect(face.style).toBe('normal');
			expect(face.weight).toBe('400');
		}
	});

	it('gives every declared face a unicode range so no subset loads for nothing', () => {
		for (const face of declaredFaces()) {
			expect(face.block).toMatch(/unicode-range:/);
		}
	});

	it('keeps the interface on the UI stack with a fallback behind each token', () => {
		expect(layoutCss).toMatch(/@import '\.\/fonts\.css';/);
		expect(layoutCss).toMatch(/--font-ui:\s*Geist,[^;]*sans-serif;/);
		expect(layoutCss).toMatch(/--font-display:\s*'Instrument Serif',[^;]*serif;/);
		expect(layoutCss).toMatch(/font-family: var\(--font-ui\);/);
		// No faux bold or slant: a weight or posture the face does not have must not be drawn.
		expect(layoutCss).toMatch(/font-synthesis: none;/);
	});
});
