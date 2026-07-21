import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { format, resolveConfig } from 'prettier';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const outputPath = path.join(root, 'src-tauri', 'resources', 'google-fonts.json');
const fixtureMode = process.argv.includes('--fixture');
const allowedLicenseRoots = new Map([
	['ofl', 'OFL-1.1'],
	['apache', 'Apache-2.0'],
	['ufl', 'UFL-1.0']
]);

const fixtureFamilies = [
	{ family: 'Inter', category: 'sans-serif', subsets: ['latin', 'latin-ext'] },
	{
		family: 'JetBrains Mono',
		category: 'monospace',
		subsets: ['cyrillic', 'latin', 'latin-ext']
	},
	{
		family: 'Noto Sans',
		category: 'sans-serif',
		subsets: ['cyrillic', 'greek', 'latin', 'latin-ext']
	},
	{ family: 'Playfair Display', category: 'serif', subsets: ['cyrillic', 'latin', 'latin-ext'] },
	{
		family: 'Roboto',
		category: 'sans-serif',
		subsets: ['cyrillic', 'greek', 'latin', 'latin-ext']
	},
	{
		family: 'Source Serif 4',
		category: 'serif',
		subsets: ['cyrillic', 'greek', 'latin', 'latin-ext']
	}
];

await loadLocalEnvironment();

const apiKey = process.env.GOOGLE_FONTS_API_KEY?.trim();
if (!fixtureMode && !apiKey) {
	throw new Error(
		'GOOGLE_FONTS_API_KEY is required. Put it in an ignored .env.local file or the current shell environment.'
	);
}

const headers = {
	Accept: 'application/vnd.github+json',
	'User-Agent': 'FontNest-catalog-builder'
};
const branch = await fetchJson('https://api.github.com/repos/google/fonts/branches/main', headers);
const sourceCommit = branch.commit.sha;
const tree = await fetchJson(
	`https://api.github.com/repos/google/fonts/git/trees/${sourceCommit}?recursive=1`,
	headers
);

if (tree.truncated) {
	throw new Error(
		'The official Google Fonts repository tree was truncated; refusing to write a partial manifest.'
	);
}

const sourceFamilies = fixtureMode
	? fixtureFamilies.map((family, index) => ({
			...family,
			lastModified: new Date().toISOString().slice(0, 10),
			version: 'fixture',
			popularityRank: index + 1,
			trendingRank: fixtureFamilies.length - index
		}))
	: await fetchGoogleFamilies(apiKey);

const treeEntries = tree.tree.filter((entry) => entry.type === 'blob');
const families = sourceFamilies
	.map((source) => buildFamily(source, sourceCommit, treeEntries))
	.filter(Boolean)
	.sort((left, right) => left.family.localeCompare(right.family));

if (!families.length) {
	throw new Error(
		'No redistributable Google Fonts families could be matched to the official repository.'
	);
}

const manifest = {
	schemaVersion: 1,
	generatedAt: new Date().toISOString(),
	sourceCommit,
	snapshot: `${fixtureMode ? 'fixture' : 'google-api'}:${sourceCommit.slice(0, 12)}`,
	families
};

await mkdir(path.dirname(outputPath), { recursive: true });
const prettierOptions = (await resolveConfig(outputPath)) ?? {};
const formattedManifest = await format(JSON.stringify(manifest), {
	...prettierOptions,
	filepath: outputPath
});
await writeFile(outputPath, formattedManifest, 'utf8');
process.stdout.write(
	`Wrote ${families.length} families and ${families.reduce((count, family) => count + family.artifacts.length, 0)} font files to ${path.relative(root, outputPath)}.\n`
);

async function loadLocalEnvironment() {
	for (const name of ['.env.local', '.env']) {
		const envPath = path.join(root, name);
		if (!existsSync(envPath)) continue;
		const contents = await readFile(envPath, 'utf8');
		for (const line of contents.split(/\r?\n/)) {
			const match = line.match(/^\s*([A-Z][A-Z0-9_]*)\s*=\s*(.*?)\s*$/);
			if (!match || process.env[match[1]]) continue;
			process.env[match[1]] = match[2].replace(/^(['"])(.*)\1$/, '$2');
		}
	}
}

async function fetchGoogleFamilies(key) {
	// Both orderings come from the same catalogue, so the rank a family holds in each
	// response is the only popularity and trending signal the API exposes. Capture it here
	// or it is lost the moment the manifest is sorted by name.
	const [popular, trending] = await Promise.all([
		fetchGoogleFamilyOrder(key, 'popularity'),
		fetchGoogleFamilyOrder(key, 'trending')
	]);
	const trendingRanks = new Map(trending.map((item, index) => [item.family, index + 1]));

	return popular.map((item, index) => ({
		...item,
		popularityRank: index + 1,
		trendingRank: trendingRanks.get(item.family) ?? null
	}));
}

async function fetchGoogleFamilyOrder(key, sort) {
	const endpoint = new URL('https://www.googleapis.com/webfonts/v1/webfonts');
	endpoint.searchParams.set('capability', 'VF');
	endpoint.searchParams.set('sort', sort);
	endpoint.searchParams.set('key', key);
	const response = await fetchJson(endpoint);
	return response.items;
}

async function fetchJson(url, requestHeaders = {}) {
	const response = await fetch(url, { headers: requestHeaders });
	if (!response.ok) {
		throw new Error(`Request failed with HTTP ${response.status}: ${new URL(url).origin}`);
	}
	return response.json();
}

function buildFamily(source, sourceCommit, entries) {
	const repositorySlug = compactSlug(source.family);
	const familyPrefix = [...allowedLicenseRoots.keys()]
		.map((licenseRoot) => `${licenseRoot}/${repositorySlug}/`)
		.find((prefix) => entries.some((entry) => entry.path.startsWith(prefix)));

	if (!familyPrefix) return null;

	const [licenseRoot] = familyPrefix.split('/');
	const rootFiles = entries.filter((entry) => {
		if (!entry.path.startsWith(familyPrefix) || !entry.path.toLowerCase().endsWith('.ttf'))
			return false;
		return entry.path.slice(familyPrefix.length).split('/').length === 1;
	});
	const fontFiles = rootFiles.length
		? rootFiles
		: entries.filter(
				(entry) =>
					entry.path.startsWith(`${familyPrefix}static/`) &&
					entry.path.toLowerCase().endsWith('.ttf')
			);
	const licenseFile = entries.find((entry) => {
		if (!entry.path.startsWith(familyPrefix)) return false;
		const relativePath = entry.path.slice(familyPrefix.length);
		return !relativePath.includes('/') && /^(OFL|LICENSE|UFL)(\.txt)?$/i.test(relativePath);
	});

	if (!fontFiles.length || !licenseFile) return null;

	const familyId = `gf:${slugify(source.family)}`;
	const artifacts = fontFiles
		.sort((left, right) => left.path.localeCompare(right.path))
		.map((entry) => ({
			id: `${familyId}:${entry.sha.slice(0, 12)}`,
			fileName: path.posix.basename(entry.path),
			style: styleFromFileName(entry.path),
			format: 'TrueType',
			downloadUrl: rawUrl(sourceCommit, entry.path),
			gitBlobSha: entry.sha,
			sizeBytes: entry.size
		}));
	const previewArtifact =
		artifacts.find((artifact) => !artifact.style.toLowerCase().includes('italic')) ??
		artifacts[0];

	return {
		id: familyId,
		family: source.family,
		category: normalizeCategory(source.category),
		subsets: [...new Set(source.subsets ?? [])].sort(),
		license: allowedLicenseRoots.get(licenseRoot),
		licenseUrl: rawUrl(sourceCommit, licenseFile.path),
		licenseGitBlobSha: licenseFile.sha,
		licenseSizeBytes: licenseFile.size,
		lastModified: source.lastModified ?? new Date().toISOString().slice(0, 10),
		version: source.version ?? 'unknown',
		popularityRank: source.popularityRank ?? null,
		trendingRank: source.trendingRank ?? null,
		previewArtifactId: previewArtifact.id,
		artifacts
	};
}

function rawUrl(commit, repositoryPath) {
	const encodedPath = repositoryPath.split('/').map(encodeURIComponent).join('/');
	return `https://raw.githubusercontent.com/google/fonts/${commit}/${encodedPath}`;
}

function compactSlug(value) {
	return value
		.normalize('NFKD')
		.toLowerCase()
		.replace(/[^a-z0-9]/g, '');
}

function slugify(value) {
	return value
		.normalize('NFKD')
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-|-$/g, '');
}

function normalizeCategory(category) {
	return String(category ?? 'display')
		.toLowerCase()
		.replaceAll('_', '-');
}

function styleFromFileName(repositoryPath) {
	const filename = path.posix.basename(repositoryPath, '.ttf');
	const axes = filename.match(/\[([^\]]+)\]/)?.[1];
	const italic = /italic/i.test(filename);
	if (axes) return italic ? `Variable Italic (${axes})` : `Variable (${axes})`;
	const suffix = filename.split('-').slice(1).join(' ');
	return suffix || 'Regular';
}
