// Resolve a theme before the first paint so the web view never flashes white. The saved theme
// lives in FontNest's own settings document now, which this script cannot read, so it resolves
// the system preference: right for anyone on the default, and a sensible guess for everyone else.
// The window itself stays hidden until the frontend has applied the real theme, so a wrong guess
// here is never something anybody sees. Loaded as a same-origin script because the packaged CSP
// (default-src 'self') blocks inline scripts.
(function () {
	try {
		var resolved = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
		var root = document.documentElement;
		root.dataset.theme = resolved;
		root.style.colorScheme = resolved;
	} catch {
		// Fall back to the CSS default theme.
	}
})();
