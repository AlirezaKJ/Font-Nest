/** @type {import("prettier").Config} */
const config = {
	useTabs: true,
	singleQuote: true,
	trailingComma: 'none',
	printWidth: 100,
	// Stated rather than left to the default, because this is the one setting a Windows checkout
	// used to disagree with. `.gitattributes` keeps the working tree LF; this keeps Prettier from
	// ever writing anything else. See `.editorconfig` for the same rule in editors.
	endOfLine: 'lf',
	plugins: ['prettier-plugin-svelte', 'prettier-plugin-tailwindcss'],
	overrides: [{ files: '*.svelte', options: { parser: 'svelte' } }],
	tailwindStylesheet: './src/routes/layout.css'
};

export default config;
