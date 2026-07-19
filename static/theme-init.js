// Resolve and apply the saved theme before the first paint so a dark-mode user
// never sees a light flash while the app hydrates. Kept in lockstep with
// applyTheme()/loadPreferences() in src/routes/+page.svelte. Loaded as a
// same-origin script because the packaged CSP (default-src 'self') blocks inline
// scripts.
(function () {
	try {
		var saved = JSON.parse(localStorage.getItem('fontnest.preferences.v1') || '{}');
		var pref =
			saved && ['system', 'light', 'dark'].indexOf(saved.theme) !== -1 ? saved.theme : 'system';
		var resolved =
			pref === 'system'
				? window.matchMedia('(prefers-color-scheme: dark)').matches
					? 'dark'
					: 'light'
				: pref;
		var root = document.documentElement;
		root.dataset.theme = resolved;
		root.style.colorScheme = resolved;
	} catch {
		// Fall back to the CSS default theme.
	}
})();
