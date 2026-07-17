export type ReorderPosition = 'before' | 'after';

export function getDirectionalReorderPosition(
	ids: readonly string[],
	draggedId: string,
	targetId: string
): ReorderPosition | null {
	const draggedIndex = ids.indexOf(draggedId);
	const targetIndex = ids.indexOf(targetId);
	if (draggedIndex < 0 || targetIndex < 0 || draggedIndex === targetIndex) return null;
	return draggedIndex < targetIndex ? 'after' : 'before';
}

export function reorderIds(
	ids: readonly string[],
	draggedId: string,
	targetId: string,
	position: ReorderPosition
): string[] {
	if (draggedId === targetId || !ids.includes(draggedId) || !ids.includes(targetId)) {
		return [...ids];
	}

	const reordered = ids.filter((id) => id !== draggedId);
	const targetIndex = reordered.indexOf(targetId);
	reordered.splice(targetIndex + (position === 'after' ? 1 : 0), 0, draggedId);

	return reordered;
}
