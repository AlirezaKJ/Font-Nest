import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { applyVersion, collectVersionProblems, repoRoot } from '../../scripts/versions.mjs';

/**
 * Files the checker reads. The Google Fonts snapshot is 2.5 MB of families the checker only reads
 * one number out of, so the fixture writes a stub instead of copying it.
 */
const COPIED_FILES = [
	'package.json',
	'CHANGELOG.md',
	'rust-toolchain.toml',
	'.github/workflows/release.yml',
	'scripts/refresh-google-fonts.mjs',
	'src-tauri/Cargo.toml',
	'src-tauri/Cargo.lock',
	'src-tauri/tauri.conf.json',
	'src-tauri/src/google_fonts.rs'
];

const fixtureRoots: string[] = [];

/** Copies the repository's real version-carrying files into a throwaway directory. */
function makeFixtureRoot(): string {
	const root = mkdtempSync(path.join(tmpdir(), 'fontnest-versions-'));
	fixtureRoots.push(root);

	for (const relative of COPIED_FILES) {
		const destination = path.join(root, relative);
		mkdirSync(path.dirname(destination), { recursive: true });
		copyFileSync(path.join(repoRoot, relative), destination);
	}

	const manifest = path.join(root, 'src-tauri/resources/google-fonts.json');
	mkdirSync(path.dirname(manifest), { recursive: true });
	writeFileSync(manifest, JSON.stringify({ schemaVersion: 1 }));

	return root;
}

/** Reads a file back out of a fixture root. */
function readFixture(root: string, relative: string): string {
	return readFileSync(path.join(root, relative), 'utf8');
}

/** Overwrites a file inside a fixture root. */
function writeFixture(root: string, relative: string, text: string): void {
	writeFileSync(path.join(root, relative), text);
}

afterEach(() => {
	for (const root of fixtureRoots.splice(0)) {
		rmSync(root, { recursive: true, force: true });
	}
});

describe('repository versions', () => {
	// This is the drift check itself, running against the real repository. A release built from a
	// tree where these disagree ships an installer whose About box, updater feed and release notes
	// do not describe the same build.
	it('carries the same version everywhere', () => {
		expect(collectVersionProblems()).toEqual([]);
	});
});

describe('collectVersionProblems', () => {
	it('reports a Cargo package version that lags package.json', () => {
		const root = makeFixtureRoot();
		const cargo = readFixture(root, 'src-tauri/Cargo.toml');
		writeFixture(
			root,
			'src-tauri/Cargo.toml',
			cargo.replace(/^version = "[^"]+"$/m, 'version = "0.0.9"')
		);

		const problems = collectVersionProblems({ root });

		expect(problems).toHaveLength(1);
		expect(problems[0].file).toBe('src-tauri/Cargo.toml');
		expect(problems[0].message).toContain('0.0.9');
	});

	it('reports a changelog with no section for the version being built', () => {
		const root = makeFixtureRoot();
		const changelog = readFixture(root, 'CHANGELOG.md');
		writeFixture(
			root,
			'CHANGELOG.md',
			changelog.replace(/^## \[0\.1\.4\].*$/m, '## [Unreleased]')
		);

		const problems = collectVersionProblems({ root });

		expect(problems).toHaveLength(1);
		expect(problems[0].file).toBe('CHANGELOG.md');
	});

	it('reports a release workflow pinned to a different Rust toolchain', () => {
		const root = makeFixtureRoot();
		const toolchain = readFixture(root, 'rust-toolchain.toml');
		writeFixture(
			root,
			'rust-toolchain.toml',
			toolchain.replace(/^channel = "[^"]+"$/m, 'channel = "1.90.0"')
		);

		const problems = collectVersionProblems({ root });

		expect(problems).toHaveLength(1);
		expect(problems[0].file).toBe('.github/workflows/release.yml');
		expect(problems[0].message).toContain('1.90.0');
	});

	it('reports a Google Fonts snapshot the parser would refuse', () => {
		const root = makeFixtureRoot();
		writeFixture(
			root,
			'src-tauri/resources/google-fonts.json',
			JSON.stringify({ schemaVersion: 7 })
		);

		const problems = collectVersionProblems({ root });

		expect(problems).toHaveLength(1);
		expect(problems[0].file).toBe('src-tauri/resources/google-fonts.json');
	});

	it('refuses a tag that does not name the version being built', () => {
		const root = makeFixtureRoot();

		expect(collectVersionProblems({ root, tag: 'v0.1.4' })).toEqual([]);
		expect(collectVersionProblems({ root, tag: 'v0.2.0' })).toEqual([
			{ file: 'git tag', message: 'v0.2.0 does not match the application version v0.1.4.' }
		]);
	});
});

describe('applyVersion', () => {
	it('moves every version file at once and leaves the changelog to a person', () => {
		const root = makeFixtureRoot();

		const written = applyVersion('0.2.0', { root });

		expect(written).toEqual([
			'package.json',
			'src-tauri/Cargo.toml',
			'src-tauri/Cargo.lock',
			'src-tauri/tauri.conf.json'
		]);
		expect(JSON.parse(readFixture(root, 'package.json')).version).toBe('0.2.0');
		expect(JSON.parse(readFixture(root, 'src-tauri/tauri.conf.json')).version).toBe('0.2.0');
		expect(readFixture(root, 'src-tauri/Cargo.toml')).toContain('version = "0.2.0"');
		// Tolerant of line endings on purpose: a Windows checkout hands the lock back with CRLF,
		// and the rewrite matches either.
		expect(readFixture(root, 'src-tauri/Cargo.lock')).toMatch(
			/name = "fontnest"\r?\nversion = "0\.2\.0"/
		);

		// The changelog is the one file it does not touch, so the check still fails until the
		// release notes have been written.
		expect(collectVersionProblems({ root }).map((problem) => problem.file)).toEqual([
			'CHANGELOG.md',
			'CHANGELOG.md',
			'CHANGELOG.md'
		]);
	});

	it('refuses a version that is not releasable', () => {
		const root = makeFixtureRoot();

		expect(() => applyVersion('nightly', { root })).toThrow(/not a version/);
	});
});
