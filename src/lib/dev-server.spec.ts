import net from 'node:net';
import { readFileSync } from 'node:fs';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { PREFERRED_PORT, devConfig, findAvailablePort } from '../../scripts/desktop.mjs';
import { repoRoot } from '../../scripts/versions.mjs';

/** Listeners opened by a test, closed again afterwards whatever the test did. */
const listeners: net.Server[] = [];

function occupy(port: number, host: string): Promise<void> {
	return new Promise((resolve, reject) => {
		const server = net.createServer();
		listeners.push(server);
		server.once('error', reject);
		server.listen({ host, port, exclusive: true }, () => resolve());
	});
}

afterEach(async () => {
	await Promise.all(
		listeners.splice(0).map(
			(server) =>
				new Promise<void>((resolve) => {
					server.close(() => resolve());
				})
		)
	);
});

describe('desktop dev server port', () => {
	it('keeps the usual port when nothing else holds it', async () => {
		const port = await findAvailablePort(45210, 5);

		expect(port).toBe(45210);
	});

	// The failure this replaces: `tauri dev` stopped with "Port 5173 is already in use" whenever a
	// second checkout, or any other Vite project, got there first.
	it('steps past a port another server is already on', async () => {
		await occupy(45220, '127.0.0.1');

		expect(await findAvailablePort(45220, 5)).toBe(45221);
	});

	// Which localhost address Vite binds depends on how the machine resolves the name, so a port
	// held on either one has to count as taken.
	it('steps past a port held on one localhost address only', async () => {
		await occupy(45230, '127.0.0.1');
		await occupy(45231, '::1');

		expect(await findAvailablePort(45230, 5)).toBe(45232);
	});

	it('says which ports it tried rather than failing silently', async () => {
		await Promise.all([occupy(45240, '127.0.0.1'), occupy(45241, '127.0.0.1')]);

		await expect(findAvailablePort(45240, 2)).rejects.toThrow(/45240 and 45241/);
	});

	// Both halves of `tauri dev` have to name one server: Vite is pinned to the port with
	// --strictPort, and the window is pointed at the same number.
	it('hands the same pinned port to Vite and to the window', () => {
		const config = devConfig(45250);

		expect(config.build.devUrl).toBe('http://localhost:45250');
		expect(config.build.beforeDevCommand).toContain('--port 45250');
		expect(config.build.beforeDevCommand).toContain('--strictPort');
	});

	it('agrees with the port the checked-in configuration names', () => {
		const viteConfig = readFileSync(path.join(repoRoot, 'vite.config.ts'), 'utf8');
		const tauriConfig = JSON.parse(
			readFileSync(path.join(repoRoot, 'src-tauri/tauri.conf.json'), 'utf8')
		) as { build?: { devUrl?: string } };

		expect(viteConfig).toMatch(new RegExp(`port: ${PREFERRED_PORT}\\b`));
		expect(tauriConfig.build?.devUrl).toBe(`http://localhost:${PREFERRED_PORT}`);
	});

	it('starts the desktop app through the launcher', () => {
		const pkg = JSON.parse(readFileSync(path.join(repoRoot, 'package.json'), 'utf8')) as {
			scripts?: Record<string, string>;
		};

		expect(pkg.scripts?.desktop).toBe('node scripts/desktop.mjs');
	});
});
