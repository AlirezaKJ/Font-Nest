import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
import type { FontFaceSummary } from '$lib/bindings/FontFaceSummary';
import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';

type BrowserFamily = {
	name: string;
	styles: Array<[string, number, 'normal' | 'italic']>;
	format: string;
	source: string;
	monospaced?: boolean;
	conflict?: boolean;
};

const browserFamilies: BrowserFamily[] = [
	{
		name: 'Segoe UI',
		styles: [
			['Regular', 400, 'normal'],
			['Semibold', 600, 'normal'],
			['Bold', 700, 'normal'],
			['Italic', 400, 'italic']
		],
		format: 'TrueType',
		source: 'System'
	},
	{
		name: 'Georgia',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal'],
			['Italic', 400, 'italic']
		],
		format: 'TrueType',
		source: 'System'
	},
	{
		name: 'Arial',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal'],
			['Italic', 400, 'italic']
		],
		format: 'TrueType',
		source: 'System'
	},
	{
		name: 'Bahnschrift',
		styles: [
			['Regular', 400, 'normal'],
			['Semi Bold', 600, 'normal']
		],
		format: 'OpenType',
		source: 'System'
	},
	{
		name: 'Consolas',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal'],
			['Italic', 400, 'italic']
		],
		format: 'TrueType',
		source: 'System',
		monospaced: true
	},
	{
		name: 'Palatino Linotype',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal']
		],
		format: 'TrueType',
		source: 'System'
	},
	{
		name: 'Trebuchet MS',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal']
		],
		format: 'TrueType',
		source: 'System'
	},
	{
		name: 'Courier New',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal']
		],
		format: 'TrueType',
		source: 'System',
		monospaced: true
	},
	{
		name: 'Source Serif 4',
		styles: [
			['Regular', 400, 'normal'],
			['Semi Bold', 600, 'normal'],
			['Bold', 700, 'normal']
		],
		format: 'OpenType',
		source: 'User'
	},
	{
		name: 'Inter',
		styles: [
			['Regular', 400, 'normal'],
			['Medium', 500, 'normal'],
			['Semi Bold', 600, 'normal'],
			['Regular', 400, 'normal']
		],
		format: 'OpenType',
		source: 'User',
		conflict: true
	}
];

function createFace(
	family: BrowserFamily,
	styleName: string,
	weight: number,
	style: 'normal' | 'italic',
	index: number
): FontFaceSummary {
	const fileStem = family.name.replaceAll(' ', '');
	const conflictSuffix = family.conflict && index === family.styles.length - 1 ? '-Copy' : '';
	return {
		id: `${family.name.toLowerCase()}:${index}`,
		postScriptName: `${fileStem}-${styleName.replaceAll(' ', '')}`,
		styleName,
		style,
		weight,
		format: family.format,
		source: family.source,
		fileName: `${fileStem}-${styleName.replaceAll(' ', '')}${conflictSuffix}.${family.format === 'OpenType' ? 'otf' : 'ttf'}`,
		faceIndex: 0,
		monospaced: family.monospaced ?? false
	};
}

function createFamily(family: BrowserFamily): FontFamilySummary {
	const faces = family.styles.map(([name, weight, style], index) =>
		createFace(family, name, weight, style, index)
	);
	return {
		id: family.name.toLowerCase(),
		name: family.name,
		faceCount: faces.length,
		fileCount: family.conflict ? 2 : 1,
		styles: faces.map((face) => face.styleName),
		weights: [...new Set(faces.map((face) => face.weight))].sort((a, b) => a - b),
		formats: [family.format],
		sources: [family.source],
		monospaced: family.monospaced ?? false,
		hasConflict: family.conflict ?? false,
		faces
	};
}

export function createBrowserCatalogue(): FontCatalogue {
	const families = browserFamilies.map(createFamily);
	return {
		families,
		familyCount: families.length,
		faceCount: families.reduce((total, family) => total + family.faceCount, 0),
		conflictCount: families.filter((family) => family.hasConflict).length,
		scanDurationMs: 0
	};
}
