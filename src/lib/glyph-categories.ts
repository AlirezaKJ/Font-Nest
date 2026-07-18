export type GlyphCategory = {
	key: string;
	label: string;
	description: string;
	codepoints: number[];
};

export type GlyphSetScope = 'basic' | 'full';

const BASIC_LATIN_START = 0x20;
const BASIC_LATIN_END = 0x7e;

type GlyphCategoryRule = Omit<GlyphCategory, 'codepoints'> & {
	matches: (character: string, codepoint: number) => boolean;
};

const LATIN = /\p{Script=Latin}/u;
const UPPERCASE = /\p{Lu}/u;
const LOWERCASE = /\p{Ll}/u;
const LETTER = /\p{L}/u;
const MARK = /\p{M}/u;
const NUMBER = /\p{N}/u;
const PUNCTUATION = /\p{P}/u;
const CURRENCY = /\p{Sc}/u;
const MATH = /\p{Sm}/u;
const SYMBOL = /\p{S}/u;
const SPACE_OR_CONTROL = /[\p{Z}\p{Cc}\p{Cf}]/u;
const PRIVATE_USE = /\p{Co}/u;
const EMOJI = /\p{Extended_Pictographic}/u;

function scriptRule(
	key: string,
	label: string,
	description: string,
	pattern: RegExp
): GlyphCategoryRule {
	return {
		key,
		label,
		description,
		matches: (character) => pattern.test(character)
	};
}

function inRanges(codepoint: number, ranges: ReadonlyArray<readonly [number, number]>): boolean {
	return ranges.some(([start, end]) => codepoint >= start && codepoint <= end);
}

const CATEGORY_RULES: GlyphCategoryRule[] = [
	{
		key: 'latin-uppercase',
		label: 'Latin uppercase',
		description: 'Capital Latin letters, including accented and extended forms',
		matches: (character) => LATIN.test(character) && UPPERCASE.test(character)
	},
	{
		key: 'latin-lowercase',
		label: 'Latin lowercase',
		description: 'Lowercase Latin letters, including accented and extended forms',
		matches: (character) => LATIN.test(character) && LOWERCASE.test(character)
	},
	{
		key: 'latin-other',
		label: 'Latin modifiers and forms',
		description: 'Latin titlecase, phonetic, modifier, and uncased characters',
		matches: (character) => LATIN.test(character)
	},
	scriptRule('greek', 'Greek and Coptic', 'Greek, polytonic, and Coptic characters', /\p{Script=Greek}/u),
	scriptRule('cyrillic', 'Cyrillic', 'Cyrillic letters and language extensions', /\p{Script=Cyrillic}/u),
	scriptRule('armenian', 'Armenian', 'Armenian letters and marks', /\p{Script=Armenian}/u),
	scriptRule('hebrew', 'Hebrew', 'Hebrew letters, points, and presentation forms', /\p{Script=Hebrew}/u),
	scriptRule('arabic', 'Arabic', 'Arabic letters, marks, digits, and presentation forms', /\p{Script=Arabic}/u),
	scriptRule('devanagari', 'Devanagari', 'Devanagari letters, marks, and digits', /\p{Script=Devanagari}/u),
	{
		key: 'south-southeast-asian',
		label: 'South and Southeast Asian scripts',
		description: 'Indic and Southeast Asian writing systems',
		matches: (character) =>
			/[\p{Script=Bengali}\p{Script=Gurmukhi}\p{Script=Gujarati}\p{Script=Oriya}\p{Script=Tamil}\p{Script=Telugu}\p{Script=Kannada}\p{Script=Malayalam}\p{Script=Sinhala}\p{Script=Thai}\p{Script=Lao}\p{Script=Khmer}\p{Script=Myanmar}]/u.test(
				character
			)
	},
	scriptRule('georgian', 'Georgian', 'Georgian letters and historic forms', /\p{Script=Georgian}/u),
	scriptRule('hiragana', 'Hiragana', 'Hiragana letters and marks', /\p{Script=Hiragana}/u),
	scriptRule('katakana', 'Katakana', 'Katakana letters and marks', /\p{Script=Katakana}/u),
	scriptRule('hangul', 'Hangul', 'Hangul syllables, jamo, and compatibility forms', /\p{Script=Hangul}/u),
	scriptRule('han', 'Han ideographs', 'CJK unified, compatibility, and extension ideographs', /\p{Script=Han}/u),
	{
		key: 'letterlike',
		label: 'Letterlike and enclosed forms',
		description: 'Letterlike symbols, enclosed characters, and mathematical alphabets',
		matches: (_character, codepoint) =>
			inRanges(codepoint, [
				[0x2100, 0x218f],
				[0x2460, 0x24ff],
				[0x1d400, 0x1d7ff]
			])
	},
	{
		key: 'other-writing-systems',
		label: 'Other writing systems',
		description: 'Letters from scripts not listed separately above',
		matches: (character) => LETTER.test(character)
	},
	{
		key: 'combining-marks',
		label: 'Combining marks',
		description: 'Accents and marks that combine with a preceding character',
		matches: (character) => MARK.test(character)
	},
	{
		key: 'numbers',
		label: 'Numbers',
		description: 'Decimal digits, fractions, numerals, and numeric symbols',
		matches: (character) => NUMBER.test(character)
	},
	{
		key: 'punctuation',
		label: 'Punctuation',
		description: 'General, typographic, and language-specific punctuation',
		matches: (character) => PUNCTUATION.test(character)
	},
	{
		key: 'currency',
		label: 'Currency',
		description: 'Currency signs and monetary symbols',
		matches: (character) => CURRENCY.test(character)
	},
	{
		key: 'arrows',
		label: 'Arrows',
		description: 'Directional, supplemental, and ornamental arrows',
		matches: (_character, codepoint) =>
			inRanges(codepoint, [
				[0x2190, 0x21ff],
				[0x27f0, 0x27ff],
				[0x2900, 0x297f],
				[0x2b00, 0x2bff],
				[0x1f800, 0x1f8ff]
			])
	},
	{
		key: 'math',
		label: 'Mathematics and operators',
		description: 'Mathematical operators, relations, and technical notation',
		matches: (character) => MATH.test(character)
	},
	{
		key: 'emoji-pictographs',
		label: 'Emoji and pictographs',
		description: 'Pictographic symbols with text or emoji presentation',
		matches: (character) => EMOJI.test(character)
	},
	{
		key: 'technical-geometric',
		label: 'Technical and geometric symbols',
		description: 'Technical marks, box drawing, blocks, shapes, and dingbats',
		matches: (_character, codepoint) =>
			inRanges(codepoint, [
				[0x2300, 0x23ff],
				[0x2500, 0x27bf]
			])
	},
	{
		key: 'other-symbols',
		label: 'Other symbols',
		description: 'Remaining modifier, musical, scientific, and ornamental symbols',
		matches: (character) => SYMBOL.test(character)
	},
	{
		key: 'private-use',
		label: 'Private use',
		description: 'Font-specific characters without standardized Unicode meaning',
		matches: (character) => PRIVATE_USE.test(character)
	},
	{
		key: 'spacing-controls',
		label: 'Spacing and controls',
		description: 'Spaces, joiners, directional controls, and other non-printing characters',
		matches: (character) => SPACE_OR_CONTROL.test(character)
	},
	{
		key: 'other-characters',
		label: 'Other characters',
		description: 'Remaining mapped Unicode characters',
		matches: () => true
	}
];

export function groupUnicodeCodepoints(codepoints: readonly number[]): GlyphCategory[] {
	const groups = new Map(CATEGORY_RULES.map((rule) => [rule.key, [] as number[]]));
	const uniqueCodepoints = [...new Set(codepoints)].sort((left, right) => left - right);

	for (const codepoint of uniqueCodepoints) {
		const character = String.fromCodePoint(codepoint);
		const rule = CATEGORY_RULES.find((candidate) => candidate.matches(character, codepoint));
		if (rule) groups.get(rule.key)?.push(codepoint);
	}

	return CATEGORY_RULES.flatMap((rule) => {
		const groupedCodepoints = groups.get(rule.key) ?? [];
		return groupedCodepoints.length
			? [{ key: rule.key, label: rule.label, description: rule.description, codepoints: groupedCodepoints }]
			: [];
	});
}

export function filterGlyphSetCodepoints(
	codepoints: readonly number[],
	scope: GlyphSetScope
): number[] {
	const uniqueCodepoints = [...new Set(codepoints)].sort((left, right) => left - right);
	return scope === 'basic'
		? uniqueCodepoints.filter(
				(codepoint) => codepoint >= BASIC_LATIN_START && codepoint <= BASIC_LATIN_END
			)
		: uniqueCodepoints;
}

export function formatCodepoint(codepoint: number): string {
	return `U+${codepoint.toString(16).toUpperCase().padStart(codepoint <= 0xffff ? 4 : 6, '0')}`;
}

export function glyphCellText(codepoint: number): string {
	const character = String.fromCodePoint(codepoint);
	if (MARK.test(character)) return `◌${character}`;
	if (SPACE_OR_CONTROL.test(character)) return formatCodepoint(codepoint);
	return character;
}

export function usesCodepointPlaceholder(codepoint: number): boolean {
	return SPACE_OR_CONTROL.test(String.fromCodePoint(codepoint));
}
