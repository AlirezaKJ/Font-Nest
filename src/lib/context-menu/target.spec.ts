import { describe, expect, it } from 'vitest';

import { describeFallbackTarget, type FallbackDescriptor } from './target';

function descriptor(overrides: Partial<FallbackDescriptor> = {}): FallbackDescriptor {
	return {
		tagName: 'DIV',
		inputType: null,
		isContentEditable: false,
		readOnly: false,
		disabled: false,
		value: '',
		selectedText: '',
		...overrides
	};
}

describe('fallback context menu target', () => {
	it('treats text inputs and textareas as editable', () => {
		expect(
			describeFallbackTarget(descriptor({ tagName: 'INPUT', inputType: 'search' })).kind
		).toBe('editable');
		expect(describeFallbackTarget(descriptor({ tagName: 'TEXTAREA' })).kind).toBe('editable');
	});

	it('leaves non-text inputs to the background menu', () => {
		expect(
			describeFallbackTarget(descriptor({ tagName: 'INPUT', inputType: 'range' })).kind
		).toBe('background');
		expect(
			describeFallbackTarget(descriptor({ tagName: 'INPUT', inputType: 'checkbox' })).kind
		).toBe('background');
	});

	it('treats a disabled field as background so no edit action is offered', () => {
		const target = describeFallbackTarget(
			descriptor({ tagName: 'INPUT', inputType: 'text', disabled: true, value: 'Inter' })
		);

		expect(target.kind).toBe('background');
	});

	it('reports what is selected inside an editable field', () => {
		const target = describeFallbackTarget(
			descriptor({
				tagName: 'INPUT',
				inputType: 'text',
				value: 'Source Serif',
				selectedText: 'Serif'
			})
		);

		expect(target).toEqual({
			kind: 'editable',
			readOnly: false,
			hasValue: true,
			selectedText: 'Serif'
		});
	});

	it('offers selection actions for highlighted text outside a field', () => {
		expect(describeFallbackTarget(descriptor({ selectedText: '  Inter Display  ' }))).toEqual({
			kind: 'selection',
			selectedText: 'Inter Display'
		});
	});

	it('ignores a whitespace-only selection', () => {
		expect(describeFallbackTarget(descriptor({ selectedText: '  \n ' })).kind).toBe(
			'background'
		);
	});

	it('falls back to the app menu on ordinary chrome', () => {
		expect(describeFallbackTarget(descriptor()).kind).toBe('background');
	});
});
