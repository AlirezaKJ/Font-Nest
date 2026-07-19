// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}

	// Injected by Vite (see vite.config.ts). The application version at build time.
	const __APP_VERSION__: string;
}

// Raw text imports (e.g. the bundled CHANGELOG.md baseline).
declare module '*.md?raw' {
	const contents: string;
	export default contents;
}

export {};
