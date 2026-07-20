import { readClipboardText, writeClipboardText } from './clipboard';

export type TextField = HTMLInputElement | HTMLTextAreaElement;

export function isTextField(element: HTMLElement): element is TextField {
	return element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement;
}

export function selectAll(element: HTMLElement) {
	element.focus();
	if (isTextField(element)) {
		element.select();
		return;
	}
	const range = document.createRange();
	range.selectNodeContents(element);
	const selection = window.getSelection();
	selection?.removeAllRanges();
	selection?.addRange(range);
}

export function selectedText(element: HTMLElement): string {
	if (isTextField(element)) {
		const start = element.selectionStart ?? 0;
		const end = element.selectionEnd ?? 0;
		return element.value.slice(start, end);
	}
	return window.getSelection()?.toString() ?? '';
}

export async function copySelection(element: HTMLElement): Promise<boolean> {
	return writeClipboardText(selectedText(element));
}

export async function cutSelection(element: HTMLElement): Promise<boolean> {
	const text = selectedText(element);
	if (!(await writeClipboardText(text))) return false;
	replaceSelection(element, '');
	return true;
}

export async function pasteIntoField(element: HTMLElement): Promise<boolean> {
	const text = await readClipboardText();
	if (text === null) return false;
	replaceSelection(element, text);
	return true;
}

export function clearField(element: HTMLElement) {
	if (isTextField(element)) {
		element.focus();
		element.value = '';
		notifyInput(element);
		return;
	}
	selectAll(element);
	replaceSelection(element, '');
}

/**
 * Replaces the current selection with `text`.
 *
 * Text fields are edited directly and told about it, because Svelte reads their value from
 * the `input` event. Rich `contenteditable` regions go through `execCommand`, which is the
 * only primitive that keeps native undo history intact.
 */
function replaceSelection(element: HTMLElement, text: string) {
	element.focus();

	if (isTextField(element)) {
		const start = element.selectionStart ?? element.value.length;
		const end = element.selectionEnd ?? start;
		element.value = element.value.slice(0, start) + text + element.value.slice(end);
		const caret = start + text.length;
		element.setSelectionRange(caret, caret);
		notifyInput(element);
		return;
	}

	if (text) document.execCommand('insertText', false, text);
	else document.execCommand('delete');
}

function notifyInput(element: TextField) {
	element.dispatchEvent(new Event('input', { bubbles: true }));
}
