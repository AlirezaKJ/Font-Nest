import type { ManagedStorageStatus } from '$lib/bindings/ManagedStorageStatus';

/**
 * What to tell someone whose last session was interrupted part way through installing fonts.
 *
 * FontNest undoes that work at startup rather than leaving fonts nothing accounts for, and this is
 * how it says so. Silence would be worse: a family that was in the middle of installing is simply
 * gone the next time they look, and nothing explains why.
 *
 * Returns an empty string when there is nothing to report, which is the ordinary case.
 */
export function managedRecoveryNotice(status: ManagedStorageStatus | null): string {
	if (!status) return '';

	const sentences: string[] = [];
	if (status.recoveredOperations === 1) {
		sentences.push('FontNest undid a font install that was interrupted last time.');
	} else if (status.recoveredOperations > 1) {
		sentences.push(
			`FontNest undid ${status.recoveredOperations} font installs that were interrupted last time.`
		);
	}
	if (status.quarantinedOperations === 1) {
		sentences.push(
			'One interrupted install could not be undone, so its files are still on this computer.'
		);
	} else if (status.quarantinedOperations > 1) {
		sentences.push(
			`${status.quarantinedOperations} interrupted installs could not be undone, so their files are still on this computer.`
		);
	}

	return sentences.join(' ');
}
