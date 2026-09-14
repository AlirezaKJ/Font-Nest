/**
 * FontNest keeps the same number in five places (package.json, Cargo.toml, Cargo.lock,
 * tauri.conf.json, CHANGELOG.md) and the release tag has to agree with all of them. Nothing
 * enforced that, so a release could ship an installer whose About box, updater feed and release
 * notes disagreed. `package.json` is the source of truth; everything else is checked against it.
 *
 * The same run also checks the pins that decide how a release is built: the Rust toolchain, the
 * pnpm version and the Node version in the release workflow have to match the ones the repository
 * declares, and the Google Fonts manifest schema has to match the parser that reads it.
 *
 * Run `node scripts/versions.mjs` to report drift, or `node scripts/versions.mjs --set 0.1.5` to
 * move every file to a new version.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

/** Absolute path of the repository root, resolved from this file rather than the shell's cwd. */
export const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

/**
 * One disagreement between two files. `file` is the copy that has to change; the message names
 * both values and where the right one lives.
 *
 * @typedef {object} VersionProblem
 * @property {string} file Repository-relative path of the file holding the wrong value.
 * @property {string} message Plain sentence naming both values and where the right one lives.
 */

const PACKAGE_JSON = 'package.json';
const CARGO_TOML = 'src-tauri/Cargo.toml';
const CARGO_LOCK = 'src-tauri/Cargo.lock';
const TAURI_CONF = 'src-tauri/tauri.conf.json';
const CHANGELOG = 'CHANGELOG.md';
const RUST_TOOLCHAIN = 'rust-toolchain.toml';
const RELEASE_WORKFLOW = '.github/workflows/release.yml';
const GOOGLE_FONTS_MANIFEST = 'src-tauri/resources/google-fonts.json';
const GOOGLE_FONTS_PARSER = 'src-tauri/src/google_fonts.rs';
const GOOGLE_FONTS_SCRIPT = 'scripts/refresh-google-fonts.mjs';

const RELEASABLE_VERSION = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;

/**
 * Reads a repository file as UTF-8 text.
 *
 * @param {string} root
 * @param {string} relative
 * @returns {string}
 */
function read(root, relative) {
	return readFileSync(path.join(root, relative), 'utf8');
}

/**
 * Escapes a string so it can be dropped into a regular expression as a literal.
 *
 * @param {string} value
 * @returns {string}
 */
function escapeForRegExp(value) {
	return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * The version FontNest is currently at. Every other version in the repository follows this one.
 *
 * @param {string} [root] Repository root to read from.
 * @returns {string}
 */
export function readApplicationVersion(root = repoRoot) {
	const manifest = /** @type {{ version?: string }} */ (JSON.parse(read(root, PACKAGE_JSON)));
	const version = manifest.version;

	if (typeof version !== 'string' || !RELEASABLE_VERSION.test(version)) {
		throw new Error(`${PACKAGE_JSON} does not carry a usable version: ${String(version)}`);
	}

	return version;
}

/**
 * Pulls the `[package]` version out of Cargo.toml, ignoring the dependency versions below it.
 *
 * @param {string} text
 * @returns {string | null}
 */
function cargoPackageVersion(text) {
	const packageSection = text.split(/^\[/m)[1] ?? '';
	return packageSection.match(/^version = "([^"]+)"$/m)?.[1] ?? null;
}

/**
 * Pulls the locked version of the `fontnest` crate out of Cargo.lock.
 *
 * @param {string} text
 * @returns {string | null}
 */
function cargoLockVersion(text) {
	return text.match(/name = "fontnest"\r?\nversion = "([^"]+)"/)?.[1] ?? null;
}

/**
 * The newest `## [x.y.z]` heading in the changelog, skipping the `[Unreleased]` section.
 *
 * @param {string} text
 * @returns {string | null}
 */
function newestReleasedVersion(text) {
	for (const line of text.split(/\r?\n/)) {
		const heading = line.match(/^## \[([^\]]+)\]/);
		if (!heading) continue;
		if (heading[1].toLowerCase() === 'unreleased') continue;
		return heading[1];
	}

	return null;
}

/**
 * The changelog is what the in-app What's New view reads, so a released version missing from it
 * shows the user nothing. Its newest released section has to be the version being built, and the
 * comparison links at the bottom have to name it too.
 *
 * @param {string} root
 * @param {string} version
 * @param {string} source
 * @returns {VersionProblem[]}
 */
function collectChangelogProblems(root, version, source) {
	/** @type {VersionProblem[]} */
	const problems = [];
	const text = read(root, CHANGELOG);

	const newest = newestReleasedVersion(text);
	if (newest !== version) {
		problems.push({
			file: CHANGELOG,
			message: `the newest released section is ${newest ?? 'missing'}, but ${source}. Rename [Unreleased] when cutting a release.`
		});
	}

	if (!new RegExp(`^\\[${escapeForRegExp(version)}\\]:`, 'm').test(text)) {
		problems.push({
			file: CHANGELOG,
			message: `there is no [${version}] comparison link at the bottom of the file.`
		});
	}

	const unreleasedLink = text.match(/^\[Unreleased\]:\s*(\S+)$/m)?.[1];
	if (unreleasedLink !== undefined && !unreleasedLink.endsWith(`v${version}...HEAD`)) {
		problems.push({
			file: CHANGELOG,
			message: `the [Unreleased] link compares against ${unreleasedLink}, which is not v${version}...HEAD.`
		});
	}

	return problems;
}

/**
 * Compares the versions carried by every file against `package.json`.
 *
 * @param {string} root
 * @param {string} version
 * @returns {VersionProblem[]}
 */
function collectApplicationVersionProblems(root, version) {
	/** @type {VersionProblem[]} */
	const problems = [];
	const source = `${PACKAGE_JSON} says ${version}`;

	const cargo = cargoPackageVersion(read(root, CARGO_TOML));
	if (cargo !== version) {
		problems.push({
			file: CARGO_TOML,
			message: `package version is ${cargo ?? 'missing'}, but ${source}.`
		});
	}

	const locked = cargoLockVersion(read(root, CARGO_LOCK));
	if (locked !== version) {
		problems.push({
			file: CARGO_LOCK,
			message: `the fontnest entry is ${locked ?? 'missing'}, but ${source}. Run a Cargo command to refresh it.`
		});
	}

	const tauri = /** @type {{ version?: string }} */ (JSON.parse(read(root, TAURI_CONF))).version;
	if (tauri !== version) {
		problems.push({
			file: TAURI_CONF,
			message: `version is ${tauri ?? 'missing'}, but ${source}.`
		});
	}

	problems.push(...collectChangelogProblems(root, version, source));

	return problems;
}

/**
 * The release workflow installs its own Rust, pnpm and Node. When those drift from the versions
 * the repository declares, a release is built by a toolchain nobody develops against.
 *
 * @param {string} root
 * @returns {VersionProblem[]}
 */
function collectToolchainProblems(root) {
	/** @type {VersionProblem[]} */
	const problems = [];
	const workflow = read(root, RELEASE_WORKFLOW);

	const declaredRust = read(root, RUST_TOOLCHAIN).match(/^channel = "([^"]+)"$/m)?.[1];
	const workflowRust = workflow.match(/^\s*toolchain:\s*(\S+)\s*$/m)?.[1];
	if (declaredRust !== undefined && workflowRust !== declaredRust) {
		problems.push({
			file: RELEASE_WORKFLOW,
			message: `it installs Rust ${workflowRust ?? 'an unread version'}, but ${RUST_TOOLCHAIN} pins ${declaredRust}.`
		});
	}

	const manifest = /** @type {{ packageManager?: string; engines?: { node?: string } }} */ (
		JSON.parse(read(root, PACKAGE_JSON))
	);

	const declaredPnpm = manifest.packageManager?.match(/^pnpm@(\d+\.\d+\.\d+)/)?.[1];
	const workflowPnpm = workflow.match(
		/action-setup@[^\n]*\n\s*with:\r?\n\s*version:\s*(\S+)\s*$/m
	)?.[1];
	if (declaredPnpm === undefined) {
		problems.push({
			file: PACKAGE_JSON,
			message: 'packageManager does not pin an exact pnpm version.'
		});
	} else if (workflowPnpm !== declaredPnpm) {
		problems.push({
			file: RELEASE_WORKFLOW,
			message: `it installs pnpm ${workflowPnpm ?? 'an unread version'}, but ${PACKAGE_JSON} pins pnpm ${declaredPnpm}.`
		});
	}

	const declaredNode = manifest.engines?.node?.match(/(\d+)/)?.[1];
	const workflowNode = workflow.match(/^\s*node-version:\s*(\S+)\s*$/m)?.[1]?.match(/(\d+)/)?.[1];
	if (declaredNode === undefined) {
		problems.push({
			file: PACKAGE_JSON,
			message: 'engines.node does not declare a supported Node major version.'
		});
	} else if (workflowNode !== declaredNode) {
		problems.push({
			file: RELEASE_WORKFLOW,
			message: `it builds on Node ${workflowNode ?? 'an unread version'}, but ${PACKAGE_JSON} declares Node ${declaredNode}.`
		});
	}

	return problems;
}

/**
 * The bundled Google Fonts snapshot is refused at runtime when its schema version is not the one
 * the Rust parser expects, so the generator, the snapshot and the parser have to agree.
 *
 * @param {string} root
 * @returns {VersionProblem[]}
 */
function collectManifestSchemaProblems(root) {
	/** @type {VersionProblem[]} */
	const problems = [];

	const parserSchema = read(root, GOOGLE_FONTS_PARSER).match(
		/const MANIFEST_SCHEMA_VERSION: u32 = (\d+);/
	)?.[1];
	if (parserSchema === undefined) {
		return [
			{
				file: GOOGLE_FONTS_PARSER,
				message:
					'MANIFEST_SCHEMA_VERSION could not be read, so the snapshot cannot be checked.'
			}
		];
	}

	const snapshot = /** @type {{ schemaVersion?: number }} */ (
		JSON.parse(read(root, GOOGLE_FONTS_MANIFEST))
	);
	if (String(snapshot.schemaVersion) !== parserSchema) {
		problems.push({
			file: GOOGLE_FONTS_MANIFEST,
			message: `schemaVersion is ${String(snapshot.schemaVersion)}, but ${GOOGLE_FONTS_PARSER} accepts ${parserSchema}.`
		});
	}

	const scriptSchema = read(root, GOOGLE_FONTS_SCRIPT).match(/schemaVersion:\s*(\d+)/)?.[1];
	if (scriptSchema !== parserSchema) {
		problems.push({
			file: GOOGLE_FONTS_SCRIPT,
			message: `it writes schemaVersion ${scriptSchema ?? 'an unread value'}, but ${GOOGLE_FONTS_PARSER} accepts ${parserSchema}.`
		});
	}

	return problems;
}

/**
 * Every version disagreement in the repository, in the order a person would fix them. An empty
 * array means the repository is consistent.
 *
 * @param {object} [options]
 * @param {string} [options.root] Repository root to read from.
 * @param {string | null} [options.tag] Git ref name to verify, as in `v0.1.5`. Release builds pass
 *   this so a mistyped tag cannot publish a mismatched installer.
 * @returns {VersionProblem[]}
 */
export function collectVersionProblems({ root = repoRoot, tag = null } = {}) {
	const version = readApplicationVersion(root);
	const problems = [
		...collectApplicationVersionProblems(root, version),
		...collectToolchainProblems(root),
		...collectManifestSchemaProblems(root)
	];

	if (tag !== null && tag !== `v${version}`) {
		problems.push({
			file: 'git tag',
			message: `${tag} does not match the application version v${version}.`
		});
	}

	return problems;
}

/**
 * Moves package.json, Cargo.toml, Cargo.lock and tauri.conf.json to `version`. The changelog is
 * left alone on purpose: renaming its [Unreleased] section is an editorial act, and
 * `collectVersionProblems` keeps failing until a person has done it.
 *
 * @param {string} version
 * @param {object} [options]
 * @param {string} [options.root]
 * @returns {string[]} Repository-relative paths that were rewritten.
 */
export function applyVersion(version, { root = repoRoot } = {}) {
	if (!RELEASABLE_VERSION.test(version)) {
		throw new Error(`${version} is not a version FontNest can release.`);
	}

	/** @type {Array<[string, RegExp, string]>} */
	const rewrites = [
		[PACKAGE_JSON, /^(\t"version": ")[^"]+(")/m, `$1${version}$2`],
		[CARGO_TOML, /^(version = ")[^"]+(")$/m, `$1${version}$2`],
		[CARGO_LOCK, /(name = "fontnest"\r?\nversion = ")[^"]+(")/, `$1${version}$2`],
		[TAURI_CONF, /^(\t"version": ")[^"]+(")/m, `$1${version}$2`]
	];

	/** @type {string[]} */
	const written = [];
	for (const [relative, pattern, replacement] of rewrites) {
		const absolute = path.join(root, relative);
		const text = readFileSync(absolute, 'utf8');
		if (!pattern.test(text)) {
			throw new Error(`${relative} does not hold a version where one was expected.`);
		}

		const updated = text.replace(pattern, replacement);
		if (updated === text) continue;

		writeFileSync(absolute, updated);
		written.push(relative);
	}

	return written;
}

/**
 * Runs the check or the rewrite, prints a report, and returns the process exit code.
 *
 * @param {string[]} argv
 * @returns {number}
 */
function runCommandLine(argv) {
	const setIndex = argv.indexOf('--set');
	if (setIndex !== -1) {
		const version = argv[setIndex + 1];
		if (version === undefined) {
			console.error('--set needs a version, as in --set 0.1.5');
			return 1;
		}

		for (const written of applyVersion(version)) {
			console.log(`updated ${written}`);
		}
		console.log(`FontNest is now ${version}. ${CHANGELOG} still has to be edited by hand.`);
		return 0;
	}

	const tagIndex = argv.indexOf('--tag');
	const tag = tagIndex === -1 ? null : (argv[tagIndex + 1] ?? null);
	if (tagIndex !== -1 && tag === null) {
		console.error('--tag needs a ref name, as in --tag v0.1.5');
		return 1;
	}

	const problems = collectVersionProblems({ tag });
	if (problems.length === 0) {
		const version = readApplicationVersion();
		console.log(
			`Versions agree: FontNest ${version}${tag === null ? '' : `, released as ${tag}`}.`
		);
		return 0;
	}

	console.error(`FontNest has ${problems.length} version disagreement(s):`);
	for (const problem of problems) {
		console.error(`  ${problem.file}: ${problem.message}`);
	}
	console.error(
		'\nEdit the files above, or run `pnpm version:set <version>` to move all of them.'
	);
	return 1;
}

const invokedDirectly =
	process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href;

if (invokedDirectly) {
	process.exitCode = runCommandLine(process.argv.slice(2));
}
