import { render, screen } from '@testing-library/svelte';
import type { ComponentProps } from 'svelte';
import { describe, expect, it, vi } from 'vitest';

import type { ImportCandidate } from '$lib/bindings/ImportCandidate';
import type { ImportOutcome } from '$lib/bindings/ImportOutcome';
import type { ImportPlan } from '$lib/bindings/ImportPlan';
import type { ImportVerdict } from '$lib/bindings/ImportVerdict';

import FontImportReview from './FontImportReview.svelte';

/**
 * The review is the last thing somebody reads before font files are written to their computer, so
 * what it declines to offer matters as much as what it shows.
 */

function candidate(verdict: ImportVerdict): ImportCandidate {
	const fileName = `${verdict}.ttf`;
	return {
		sourcePath: `C:/fonts/${fileName}`,
		fileName,
		sizeBytes: 42_000,
		verdict,
		installedFileName: verdict === 'installable' ? 'FontNest-0123456789ab-One.ttf' : null,
		faces: [],
		licence: null
	};
}

function plan(candidates: ImportCandidate[], truncated = false): ImportPlan {
	return { candidates, installableBytes: 0, truncated };
}

function button(name: string | RegExp): HTMLButtonElement {
	return screen.getByRole('button', { name }) as HTMLButtonElement;
}

function mount(overrides: Partial<ComponentProps<typeof FontImportReview>> = {}) {
	const onConfirm = vi.fn();
	const onClose = vi.fn();
	render(FontImportReview, {
		plan: plan([candidate('installable')]),
		outcomes: null,
		importing: false,
		onConfirm,
		onClose,
		...overrides
	});
	return { onConfirm, onClose };
}

describe('reviewing fonts before importing them', () => {
	it('offers to install only the files it found installable', () => {
		mount({ plan: plan([candidate('installable'), candidate('previewOnly')]) });

		expect(button('Install 1 font').disabled).toBe(false);
		expect(screen.getByText('2 files, 1 ready to install.')).toBeDefined();
	});

	// Nothing to install means nothing to confirm. An enabled button here would offer an action
	// that cannot happen.
	it('will not offer to install a selection with nothing installable in it', () => {
		mount({ plan: plan([candidate('previewOnly'), candidate('unreadable')]) });

		expect(button(/^Install/).disabled).toBe(true);
		expect(screen.getByText('2 files, none of which can be installed.')).toBeDefined();
	});

	it('says what it decided about each file, in the words the decision has everywhere else', () => {
		mount({ plan: plan([candidate('systemOwned'), candidate('alreadyInstalled')]) });

		expect(screen.getByText('Belongs to Windows')).toBeDefined();
		expect(screen.getByText('Already installed')).toBeDefined();
	});

	// While an import runs the computer is being changed, so nothing offers to start it again or
	// to close the panel out from under it.
	it('offers no way out while an import is running', () => {
		mount({ importing: true });

		expect(button(/Installing/).disabled).toBe(true);
		expect(button('Cancel').disabled).toBe(true);
		expect(button('Close font import').disabled).toBe(true);
	});

	it('asks before installing anything rather than on its way past', () => {
		const { onConfirm } = mount();

		expect(onConfirm).not.toHaveBeenCalled();
		button('Install 1 font').click();
		expect(onConfirm).toHaveBeenCalledOnce();
	});

	it('turns into a report once the import has run, with nothing left to install', () => {
		const outcomes: ImportOutcome[] = [
			{
				sourcePath: 'C:/fonts/One.ttf',
				fileName: 'One.ttf',
				installed: true,
				displayName: 'One Regular',
				refusal: null,
				failure: null
			},
			{
				sourcePath: 'C:/fonts/Two.ttf',
				fileName: 'Two.ttf',
				installed: false,
				displayName: null,
				refusal: null,
				failure: 'the registry refused the font'
			}
		];
		mount({ outcomes });

		expect(screen.getByRole('heading', { name: 'Import finished' })).toBeDefined();
		expect(screen.getByText('1 font installed, 1 file could not be.')).toBeDefined();
		expect(screen.getByText('the registry refused the font')).toBeDefined();
		expect(screen.queryByRole('button', { name: /^Install/ })).toBeNull();
	});

	// A selection read only as far as the limit would otherwise look like the whole of it.
	it('says when a selection was cut off rather than read whole', () => {
		mount({ plan: plan([candidate('installable')], true) });

		expect(screen.getByRole('status').textContent).toContain(
			'more files than FontNest reviews at once'
		);
	});
});
