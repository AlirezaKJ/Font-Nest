import { createRequire } from 'node:module';

import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vitest/config';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';

const { version } = createRequire(import.meta.url)('./package.json') as { version: string };

export default defineConfig({
	define: {
		// The running application version, kept in lockstep with Cargo and Tauri at release time.
		__APP_VERSION__: JSON.stringify(version)
	},
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			adapter: adapter({ fallback: 'index.html' })
		})
	],
	server: {
		// A preference, not a requirement: `pnpm dev` on its own moves to the next free port and
		// says where it landed, so a second checkout or a stray server does not block it. The case
		// that must not drift is `pnpm desktop`, because Tauri loads a fixed URL and would happily
		// open whatever else answers there. That launcher picks the port and passes both `--port`
		// and `--strictPort`, so the two halves always name the same server.
		port: 5173,
		watch: {
			// Tauri watches Rust sources. Vite only needs to watch the frontend and must
			// not touch locked Windows build artifacts under src-tauri/target.
			ignored: ['**/src-tauri/**']
		}
	},
	test: {
		expect: { requireAssertions: true },
		projects: [
			{
				extends: './vite.config.ts',
				test: {
					name: 'server',
					environment: 'node',
					include: ['src/**/*.{test,spec}.{js,ts}'],
					exclude: ['src/**/*.svelte.{test,spec}.{js,ts}']
				}
			}
		]
	}
});
