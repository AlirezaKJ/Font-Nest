export type ConflictNavigationIntent = 'review' | 'inspect';
export type ConflictNavigationDestination = 'duplicates' | 'library';

export function getConflictDestination(
	intent: ConflictNavigationIntent
): ConflictNavigationDestination {
	return intent === 'review' ? 'duplicates' : 'library';
}
