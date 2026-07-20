/** What the app-wide fallback menu found under the pointer. */
export type FallbackTarget =
	| { kind: 'editable'; readOnly: boolean; hasValue: boolean; selectedText: string }
	| { kind: 'selection'; selectedText: string }
	| { kind: 'background' };

export type FallbackDescriptor = {
	tagName: string;
	inputType: string | null;
	isContentEditable: boolean;
	readOnly: boolean;
	disabled: boolean;
	value: string;
	selectedText: string;
};

const TEXT_INPUT_TYPES = new Set([
	'',
	'email',
	'number',
	'password',
	'search',
	'tel',
	'text',
	'url'
]);

/**
 * Classifies a right-click that no element claimed. Kept free of DOM types so the rules
 * stay readable and testable; `describeEventTarget` supplies the descriptor at runtime.
 */
export function describeFallbackTarget(descriptor: FallbackDescriptor): FallbackTarget {
	const selectedText = descriptor.selectedText.trim();

	if (isEditable(descriptor)) {
		return {
			kind: 'editable',
			readOnly: descriptor.readOnly,
			hasValue: descriptor.value.length > 0,
			selectedText
		};
	}

	if (selectedText) return { kind: 'selection', selectedText };
	return { kind: 'background' };
}

function isEditable(descriptor: FallbackDescriptor): boolean {
	if (descriptor.disabled) return false;
	const tagName = descriptor.tagName.toLowerCase();
	if (tagName === 'textarea') return true;
	if (tagName === 'input') {
		return TEXT_INPUT_TYPES.has((descriptor.inputType ?? '').toLowerCase());
	}
	return descriptor.isContentEditable;
}

/** Reads a real event target into the descriptor `describeFallbackTarget` expects. */
export function describeEventTarget(
	target: EventTarget | null,
	selectedText: string
): FallbackDescriptor {
	const element = target instanceof HTMLElement ? target : null;
	const field =
		element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement
			? element
			: null;

	return {
		tagName: element?.tagName ?? '',
		inputType: element instanceof HTMLInputElement ? element.type : null,
		isContentEditable: element?.isContentEditable ?? false,
		readOnly: field?.readOnly ?? false,
		disabled: field?.disabled ?? false,
		value: field ? field.value : (element?.textContent ?? ''),
		selectedText
	};
}

/** Text the user has highlighted, or an empty string. */
export function currentSelectionText(): string {
	return window.getSelection()?.toString() ?? '';
}
