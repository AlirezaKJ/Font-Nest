/**
 * Parser and helpers for the FontNest CHANGELOG.md, which follows the
 * "Keep a Changelog" convention (https://keepachangelog.com/en/1.1.0/).
 *
 * FontNest reads a single CHANGELOG.md — bundled at build time and refreshed from GitHub —
 * to render its in-app release notes, so the parsing lives here and is unit-tested rather than
 * duplicated between the desktop and browser code paths.
 */

export type ChangeKind =
	'added' | 'changed' | 'deprecated' | 'removed' | 'fixed' | 'security' | 'other';

export type ReleaseSection = {
	kind: ChangeKind;
	/** Display heading exactly as authored, e.g. "Added". */
	title: string;
	/** Item text, with inline `**bold**` markers preserved for the view to render. */
	items: string[];
};

export type ReleaseEntry = {
	/** Parsed semantic version, or `null` for the "Unreleased" entry. */
	version: string | null;
	/** Heading label, e.g. "0.1.0" or "Unreleased". */
	label: string;
	/** ISO date (YYYY-MM-DD) when present in the heading. */
	date: string | null;
	yanked: boolean;
	/** Free-text paragraph authored under the version heading, before any section. */
	summary: string;
	sections: ReleaseSection[];
};

export type Changelog = {
	entries: ReleaseEntry[];
};

export type ReleaseStatus = 'unreleased' | 'newer' | 'current' | 'older';

const SECTION_KINDS: Record<string, ChangeKind> = {
	added: 'added',
	changed: 'changed',
	deprecated: 'deprecated',
	removed: 'removed',
	fixed: 'fixed',
	security: 'security'
};

const HEADING_RELEASE = /^##\s+(?!#)/;
const HEADING_SECTION = /^###\s+/;
const LIST_ITEM = /^[-*]\s+/;
const LINK_REFERENCE = /^\[[^\]]+\]:\s/;
const ISO_DATE = /\d{4}-\d{2}-\d{2}/;

function sectionKind(title: string): ChangeKind {
	return SECTION_KINDS[title.trim().toLowerCase()] ?? 'other';
}

function parseReleaseHeading(
	heading: string
): Pick<ReleaseEntry, 'version' | 'label' | 'date' | 'yanked'> {
	let rest = heading.replace(HEADING_RELEASE, '').trim();

	const yanked = /\[yanked\]/i.test(rest);
	rest = rest.replace(/\[yanked\]/i, '').trim();

	const date = rest.match(ISO_DATE)?.[0] ?? null;

	// Everything up to the first " - " (the date separator) is the version token.
	const separator = rest.indexOf(' - ');
	const versionToken = (separator === -1 ? rest : rest.slice(0, separator)).trim();
	const label = versionToken.replace(/^\[/, '').replace(/\]$/, '').trim();

	const isUnreleased = /^unreleased$/i.test(label);
	const version = isUnreleased ? null : (label.match(/\d+(?:\.\d+){1,2}[^\s]*/)?.[0] ?? null);

	return { version, label: isUnreleased ? 'Unreleased' : label, date, yanked };
}

/**
 * Parses CHANGELOG.md text into ordered release entries. Content before the first `##` heading
 * (the title and preamble) and trailing link-reference definitions are ignored.
 */
export function parseChangelog(markdown: string): Changelog {
	const entries: ReleaseEntry[] = [];
	let entry: ReleaseEntry | null = null;
	let section: ReleaseSection | null = null;
	let inItem = false;
	let summaryLines: string[] = [];

	const flushSummary = () => {
		if (entry && summaryLines.length) {
			entry.summary = summaryLines.join(' ').replace(/\s+/g, ' ').trim();
		}
		summaryLines = [];
	};

	for (const raw of markdown.split(/\r?\n/)) {
		const line = raw.trimEnd();
		const trimmed = line.trim();

		if (HEADING_SECTION.test(line)) {
			flushSummary();
			if (!entry) continue;
			const title = line.replace(HEADING_SECTION, '').trim();
			section = { kind: sectionKind(title), title, items: [] };
			entry.sections.push(section);
			inItem = false;
			continue;
		}

		if (HEADING_RELEASE.test(line)) {
			flushSummary();
			entry = { ...parseReleaseHeading(line), summary: '', sections: [] };
			entries.push(entry);
			section = null;
			inItem = false;
			continue;
		}

		if (!entry) continue;

		if (LIST_ITEM.test(trimmed)) {
			if (!section) {
				section = { kind: 'other', title: 'Notes', items: [] };
				entry.sections.push(section);
			}
			flushSummary();
			section.items.push(trimmed.replace(LIST_ITEM, '').trim());
			inItem = true;
			continue;
		}

		if (trimmed === '') {
			inItem = false;
			continue;
		}

		if (LINK_REFERENCE.test(trimmed)) continue;

		// A wrapped continuation of the current list item.
		if (inItem && section) {
			const items = section.items;
			items[items.length - 1] = `${items[items.length - 1]} ${trimmed}`.replace(/\s+/g, ' ');
			continue;
		}

		// Otherwise it is summary prose under the version heading.
		summaryLines.push(trimmed);
	}

	flushSummary();
	return { entries };
}

/**
 * Compares two dotted version strings. Returns a negative number when `a` precedes `b`, a
 * positive number when it follows, and zero when they are equal. A version carrying a
 * pre-release suffix (e.g. `1.0.0-rc.1`) sorts before its release.
 */
export function compareVersions(a: string, b: string): number {
	const parse = (value: string) => {
		const [core, prerelease = ''] = value.trim().replace(/^v/i, '').split('-', 2);
		const numbers = core.split('.').map((part) => Number.parseInt(part, 10) || 0);
		return { numbers, prerelease };
	};

	const left = parse(a);
	const right = parse(b);
	const length = Math.max(left.numbers.length, right.numbers.length);

	for (let index = 0; index < length; index += 1) {
		const diff = (left.numbers[index] ?? 0) - (right.numbers[index] ?? 0);
		if (diff !== 0) return diff < 0 ? -1 : 1;
	}

	if (left.prerelease === right.prerelease) return 0;
	if (left.prerelease === '') return 1;
	if (right.prerelease === '') return -1;
	return left.prerelease < right.prerelease ? -1 : 1;
}

/** Classifies a release relative to the version currently running. */
export function releaseStatus(entry: ReleaseEntry, currentVersion: string | null): ReleaseStatus {
	if (entry.version === null) return 'unreleased';
	if (!currentVersion) return 'older';
	const order = compareVersions(entry.version, currentVersion);
	if (order === 0) return 'current';
	return order > 0 ? 'newer' : 'older';
}
