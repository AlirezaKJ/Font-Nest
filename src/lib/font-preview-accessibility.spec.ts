import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

describe('font preview accessibility', () => {
	it('labels the metrics chart without a native SVG hover tooltip', () => {
		const componentPath = new URL('./components/FontPreviewView.svelte', import.meta.url);
		const source = readFileSync(componentPath, 'utf8');

		expect(source).toContain(
			'aria-label={`${selectedGlyph} ${glyphViewMode} view aligned to the font\'s cap height, x-height, baseline, and descender`}'
		);
		expect(source).not.toContain(
			'<title>{selectedGlyph} font metrics in {glyphViewMode} view</title>'
		);
	});
});
