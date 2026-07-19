import { describe, expect, it } from 'vitest';

import { compareVersions, parseChangelog, releaseStatus } from './changelog';

const SAMPLE = `# Changelog

Some preamble that should be ignored.

## [Unreleased]

### Added

- A brand-new thing.

## [0.2.0] - 2026-08-01

A short summary of the release
that wraps across two lines.

### Added

- **Discover** gained saved searches.
- A second added item.

### Fixed

- Corrected a preview crash that happened
  when a face was missing.

## [0.1.0] - 2026-07-18 [YANKED]

### Security

- Verified font downloads against the trusted catalogue.

[0.2.0]: https://example.com/compare/v0.1.0...v0.2.0
[0.1.0]: https://example.com/releases/tag/v0.1.0
`;

describe('parseChangelog', () => {
	const { entries } = parseChangelog(SAMPLE);

	it('parses each release heading in document order', () => {
		expect(entries.map((entry) => entry.label)).toEqual(['Unreleased', '0.2.0', '0.1.0']);
	});

	it('treats Unreleased as having no concrete version', () => {
		expect(entries[0].version).toBeNull();
		expect(entries[0].sections[0]).toMatchObject({ kind: 'added', title: 'Added' });
	});

	it('extracts version, date, and summary paragraph', () => {
		expect(entries[1].version).toBe('0.2.0');
		expect(entries[1].date).toBe('2026-08-01');
		expect(entries[1].summary).toBe(
			'A short summary of the release that wraps across two lines.'
		);
	});

	it('groups items under their sections and preserves inline markers', () => {
		const added = entries[1].sections.find((section) => section.kind === 'added');
		expect(added?.items).toEqual([
			'**Discover** gained saved searches.',
			'A second added item.'
		]);
	});

	it('joins wrapped continuation lines into a single item', () => {
		const fixed = entries[1].sections.find((section) => section.kind === 'fixed');
		expect(fixed?.items).toEqual([
			'Corrected a preview crash that happened when a face was missing.'
		]);
	});

	it('flags yanked releases and ignores link-reference definitions', () => {
		expect(entries[2].yanked).toBe(true);
		expect(entries[2].sections).toHaveLength(1);
		expect(entries[2].sections[0].kind).toBe('security');
	});

	it('ignores content before the first release heading', () => {
		expect(entries.some((entry) => entry.summary.includes('preamble'))).toBe(false);
	});

	it('returns no entries for an empty document', () => {
		expect(parseChangelog('').entries).toEqual([]);
	});
});

describe('compareVersions', () => {
	it('orders by numeric precedence', () => {
		expect(compareVersions('0.1.0', '0.1.1')).toBeLessThan(0);
		expect(compareVersions('0.2.0', '0.1.9')).toBeGreaterThan(0);
		expect(compareVersions('1.0.0', '1.0.0')).toBe(0);
	});

	it('tolerates a leading v and uneven segment counts', () => {
		expect(compareVersions('v1.2', '1.2.0')).toBe(0);
	});

	it('sorts pre-releases before their final release', () => {
		expect(compareVersions('1.0.0-rc.1', '1.0.0')).toBeLessThan(0);
	});
});

describe('releaseStatus', () => {
	const { entries } = parseChangelog(SAMPLE);

	it('marks the running version as current and newer/older around it', () => {
		expect(releaseStatus(entries[0], '0.1.0')).toBe('unreleased');
		expect(releaseStatus(entries[1], '0.1.0')).toBe('newer');
		expect(releaseStatus(entries[2], '0.1.0')).toBe('current');
	});

	it('treats every release as history when the current version is unknown', () => {
		expect(releaseStatus(entries[1], null)).toBe('older');
	});
});
