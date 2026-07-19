import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import { contrastRatio, extractColorTokens, parseHex } from './contrast';

const css = readFileSync(new URL('../../routes/layout.css', import.meta.url), 'utf8');

const light = extractColorTokens(css, /:root,\s*:root\[data-theme='light'\]\s*\{([\s\S]*?)\}/);
const darkOverrides = extractColorTokens(css, /:root\[data-theme='dark'\]\s*\{([\s\S]*?)\}/);
// Dark inherits every light token it does not override.
const dark = { ...light, ...darkOverrides };

// AA for text below 18pt (or 14pt bold). Every label in the app is small text,
// so we hold all foreground text tokens to the normal-text threshold.
const AA_NORMAL_TEXT = 4.5;

// Foreground text tokens ranked primary -> tertiary.
const FOREGROUNDS = ['--color-text', '--color-muted', '--color-subtle'];

// Surfaces text can render on. Transient state fills (hover, selected) are
// excluded because text over them uses the primary token.
const SURFACES = [
	'--color-bg',
	'--color-surface',
	'--color-panel',
	'--color-control',
	'--color-raised'
];

const themes = [
	['light', light],
	['dark', dark]
] as const;

describe('design token contrast (WCAG 2.2 AA)', () => {
	for (const [themeName, tokens] of themes) {
		describe(themeName, () => {
			for (const foreground of FOREGROUNDS) {
				for (const surface of SURFACES) {
					it(`${foreground} on ${surface} meets ${AA_NORMAL_TEXT}:1`, () => {
						const fg = parseHex(tokens[foreground]);
						const bg = parseHex(tokens[surface]);
						expect(
							fg,
							`unresolved token ${foreground} (${tokens[foreground]})`
						).not.toBeNull();
						expect(
							bg,
							`unresolved token ${surface} (${tokens[surface]})`
						).not.toBeNull();

						const ratio = contrastRatio(fg!, bg!);
						expect(
							ratio,
							`${foreground} on ${surface} is ${ratio.toFixed(2)}:1 (need ${AA_NORMAL_TEXT}:1)`
						).toBeGreaterThanOrEqual(AA_NORMAL_TEXT);
					});
				}
			}
		});
	}
});
