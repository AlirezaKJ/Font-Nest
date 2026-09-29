/**
 * Starts the desktop app in development.
 *
 * `tauri dev` starts the Vite server itself and then loads whatever answers at `devUrl`, so the
 * two have to name the same port. Both used to be written down as 5173, which meant anything else
 * already holding that port stopped the app from starting: a second FontNest checkout, a dev
 * server left running in another terminal, or any other Vite project on the machine.
 *
 * Moving the port is not enough on its own. If Vite were simply allowed to pick the next free one,
 * Tauri would still load port 5173, which is by definition somebody else's application. So this
 * picks a free port first, then hands the same number to both sides and pins it: Vite is told
 * `--strictPort`, so a mismatch fails loudly here instead of opening another project's page inside
 * FontNest.
 *
 * Run `pnpm desktop`. Every argument is passed through to `tauri dev`.
 */
import { createRequire } from 'node:module';
import net from 'node:net';
import { pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);

/** The port to use when it is free, so the familiar URL stays the usual one. */
export const PREFERRED_PORT = 5173;

/** How many consecutive ports to try before giving up. */
const PORT_ATTEMPTS = 20;

/**
 * Both localhost addresses. A server bound to only one of them still makes the port unusable for
 * the other, and which one Vite gets depends on how the machine resolves `localhost`, so a port
 * only counts as free when both are.
 */
const LOCAL_HOSTS = ['127.0.0.1', '::1'];

/**
 * Whether a listener can bind `host:port` right now.
 *
 * @param {string} host
 * @param {number} port
 * @returns {Promise<boolean>}
 */
function canBind(host, port) {
	return new Promise((resolve) => {
		const server = net.createServer();
		server.unref();
		server.once('error', (/** @type {NodeJS.ErrnoException} */ error) => {
			// A machine with IPv6 disabled cannot answer for ::1 at all, which says nothing about
			// whether the port is taken.
			resolve(error.code === 'EAFNOSUPPORT' || error.code === 'EADDRNOTAVAIL');
		});
		server.listen({ host, port, exclusive: true }, () => {
			server.close(() => resolve(true));
		});
	});
}

/**
 * The first free port at or above `start`.
 *
 * @param {number} [start]
 * @param {number} [attempts]
 * @returns {Promise<number>}
 */
export async function findAvailablePort(start = PREFERRED_PORT, attempts = PORT_ATTEMPTS) {
	for (let port = start; port < start + attempts; port += 1) {
		const free = await Promise.all(LOCAL_HOSTS.map((host) => canBind(host, port)));
		if (free.every(Boolean)) return port;
	}

	throw new Error(
		`No free port between ${start} and ${start + attempts - 1}. Stop one of the servers ` +
			'already running and try again.'
	);
}

/**
 * The partial Tauri configuration that points both halves of `tauri dev` at one port.
 *
 * @param {number} port
 * @returns {{ build: { devUrl: string, beforeDevCommand: string } }}
 */
export function devConfig(port) {
	return {
		build: {
			devUrl: `http://localhost:${port}`,
			beforeDevCommand: `pnpm dev --port ${port} --strictPort`
		}
	};
}

/**
 * @param {string[]} passthrough Arguments forwarded to `tauri dev`.
 * @returns {Promise<void>}
 */
async function main(passthrough) {
	const port = await findAvailablePort();
	if (port !== PREFERRED_PORT) {
		console.log(
			`Port ${PREFERRED_PORT} is in use, so the dev server is on ${port} instead. ` +
				'FontNest will load from there.'
		);
	}

	const cli = require('@tauri-apps/cli');
	// An array of arguments, so the configuration is never re-parsed by a shell on its way in.
	await cli.run(
		['dev', '--config', JSON.stringify(devConfig(port)), ...passthrough],
		'pnpm desktop'
	);
}

const invokedDirectly =
	process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href;

if (invokedDirectly) {
	main(process.argv.slice(2)).catch((error) => {
		console.error(error instanceof Error ? error.message : error);
		process.exitCode = 1;
	});
}
