export function isStickySurfaceElevated(
	scrollTop: number,
	controlsOffsetTop: number,
	stickyInset = 0
): boolean {
	return scrollTop + stickyInset >= controlsOffsetTop - 1;
}
