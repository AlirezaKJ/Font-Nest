import { describe, expect, it } from 'vitest';

import type { ManagedStorageStatus } from '$lib/bindings/ManagedStorageStatus';

import { managedRecoveryNotice } from './recovery-notice';

function status(overrides: Partial<ManagedStorageStatus> = {}): ManagedStorageStatus {
	return {
		writable: true,
		reason: null,
		recoveredOperations: 0,
		quarantinedOperations: 0,
		...overrides
	};
}

describe('managedRecoveryNotice', () => {
	it('says nothing when the last session finished what it started', () => {
		expect(managedRecoveryNotice(status())).toBe('');
		expect(managedRecoveryNotice(null)).toBe('');
	});

	it('explains what an interrupted session left behind and what happened to it', () => {
		expect(managedRecoveryNotice(status({ recoveredOperations: 1 }))).toBe(
			'FontNest undid a font install that was interrupted last time.'
		);
		expect(managedRecoveryNotice(status({ recoveredOperations: 3 }))).toBe(
			'FontNest undid 3 font installs that were interrupted last time.'
		);
	});

	it('does not hide files it could not clean up', () => {
		expect(managedRecoveryNotice(status({ quarantinedOperations: 1 }))).toBe(
			'One interrupted install could not be undone, so its files are still on this computer.'
		);
		expect(managedRecoveryNotice(status({ quarantinedOperations: 2 }))).toBe(
			'2 interrupted installs could not be undone, so their files are still on this computer.'
		);
	});

	it('reports both outcomes together when a session had each', () => {
		expect(
			managedRecoveryNotice(status({ recoveredOperations: 2, quarantinedOperations: 1 }))
		).toBe(
			'FontNest undid 2 font installs that were interrupted last time. One interrupted install could not be undone, so its files are still on this computer.'
		);
	});
});
