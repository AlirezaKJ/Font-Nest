import type { FontCatalogue } from '$lib/bindings/FontCatalogue';
import type { FontFaceSummary } from '$lib/bindings/FontFaceSummary';
import type { FontFamilySummary } from '$lib/bindings/FontFamilySummary';
import type { FontOrigin } from '$lib/bindings/FontOrigin';
import { sortOrigins } from '$lib/fonts/font-origin';

type BrowserFamily = {
	name: string;
	styles: Array<[string, number, 'normal' | 'italic']>;
	format: string;
	origin: FontOrigin;
	variable?: boolean;
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
		origin: 'systemDefault'
	},
	{
		name: 'Georgia',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal'],
			['Italic', 400, 'italic']
		],
		format: 'TrueType',
		origin: 'systemDefault'
	},
	{
		name: 'Arial',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal'],
			['Italic', 400, 'italic']
		],
		format: 'TrueType',
		origin: 'systemDefault'
	},
	{
		name: 'Bahnschrift',
		styles: [
			['Regular', 400, 'normal'],
			['Semi Bold', 600, 'normal']
		],
		format: 'OpenType',
		origin: 'systemDefault',
		variable: true
	},
	{
		name: 'Consolas',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal'],
			['Italic', 400, 'italic']
		],
		format: 'TrueType',
		origin: 'systemDefault',
		monospaced: true
	},
	{
		name: 'Palatino Linotype',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal']
		],
		format: 'TrueType',
		origin: 'systemDefault'
	},
	{
		name: 'Trebuchet MS',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal']
		],
		format: 'TrueType',
		origin: 'systemDefault'
	},
	{
		name: 'Courier New',
		styles: [
			['Regular', 400, 'normal'],
			['Bold', 700, 'normal']
		],
		format: 'TrueType',
		origin: 'systemDefault',
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
		origin: 'userInstalled',
		variable: true
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
		origin: 'userInstalled',
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
	const isDuplicate = family.conflict === true && index === family.styles.length - 1;
	const conflictSuffix = isDuplicate ? '-Copy' : '';
	// The duplicate stands in for the usual cause of a conflict: the same face installed
	// once for everyone and again for one account.
	const origin: FontOrigin = isDuplicate ? 'machineInstalled' : family.origin;
	return {
		id: `${family.name.toLowerCase()}:${index}`,
		postScriptName: `${fileStem}-${styleName.replaceAll(' ', '')}`,
		styleName,
		style,
		weight,
		format: family.format,
		origin,
		fileName: `${fileStem}-${styleName.replaceAll(' ', '')}${conflictSuffix}.${family.format === 'OpenType' ? 'otf' : 'ttf'}`,
		faceIndex: 0,
		monospaced: family.monospaced ?? false,
		variable: family.variable ?? false
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
		origins: sortOrigins([...new Set(faces.map((face) => face.origin))]),
		monospaced: family.monospaced ?? false,
		variable: faces.some((face) => face.variable),
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
