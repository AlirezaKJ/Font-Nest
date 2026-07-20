<script lang="ts" module>
	export type IconName =
		| 'alert'
		| 'bookmark'
		| 'check'
		| 'chevron'
		| 'close'
		| 'copy'
		| 'cut'
		| 'duplicates'
		| 'filter'
		| 'folder'
		| 'font'
		| 'library'
		| 'maximize'
		| 'minimize'
		| 'moon'
		| 'monitor'
		| 'more'
		| 'paste'
		| 'refresh'
		| 'restore'
		| 'search'
		| 'select-all'
		| 'settings'
		| 'sidebar'
		| 'sparkle'
		| 'sun'
		| 'upload';
</script>

<script lang="ts">
	let { name, size = 18 }: { name: IconName; size?: number } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width="1.7"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	{#if name === 'library'}
		<path d="M5 5.75h14v12.5H5z" />
		<path d="M8.5 9.25h7M8.5 12h4.5M8.5 14.75h6" />
	{:else if name === 'bookmark'}
		<path d="M6.5 4.5h11v15L12 16.25 6.5 19.5z" />
	{:else if name === 'duplicates'}
		<rect x="4.5" y="7.5" width="12" height="12" rx="2" />
		<path
			d="M8 7.5V6.25A1.75 1.75 0 0 1 9.75 4.5h8A1.75 1.75 0 0 1 19.5 6.25v8A1.75 1.75 0 0 1 17.75 16H16.5"
		/>
	{:else if name === 'settings'}
		<circle cx="12" cy="12" r="3" />
		<path
			d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.09a2 2 0 0 1 1 1.74v.5a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.09a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2Z"
		/>
	{:else if name === 'search'}
		<circle cx="10.8" cy="10.8" r="5.8" />
		<path d="m15.1 15.1 4.1 4.1" />
	{:else if name === 'filter'}
		<path d="M4.5 6.5h15M7.5 12h9M10 17.5h4" />
	{:else if name === 'refresh'}
		<path d="M18.25 8.25V4.8l-2.1 2.1a7 7 0 1 0 1.35 8.45" />
	{:else if name === 'upload'}
		<path d="M12 15.5V4.75M8.25 8.5 12 4.75l3.75 3.75" />
		<path d="M5 14.5v3.25A1.75 1.75 0 0 0 6.75 19.5h10.5A1.75 1.75 0 0 0 19 17.75V14.5" />
	{:else if name === 'sun'}
		<circle cx="12" cy="12" r="3.25" />
		<path
			d="M12 3.75v1.5M12 18.75v1.5M20.25 12h-1.5M5.25 12h-1.5M17.83 6.17l-1.06 1.06M7.23 16.77l-1.06 1.06M17.83 17.83l-1.06-1.06M7.23 7.23 6.17 6.17"
		/>
	{:else if name === 'moon'}
		<path d="M19.25 14.75A7.4 7.4 0 0 1 9.2 4.7a7.5 7.5 0 1 0 10.05 10.05Z" />
	{:else if name === 'monitor'}
		<rect x="4" y="5" width="16" height="11.5" rx="1.75" />
		<path d="M9 20h6M12 16.5V20" />
	{:else if name === 'check'}
		<path d="m5.5 12 4 4 9-9" />
	{:else if name === 'alert'}
		<path
			d="M10.35 4.8 3.8 16.1a1.75 1.75 0 0 0 1.52 2.65h13.36a1.75 1.75 0 0 0 1.52-2.65L13.65 4.8a1.9 1.9 0 0 0-3.3 0Z"
		/>
		<path d="M12 9v3.5M12 16h.01" />
	{:else if name === 'close'}
		<path d="m7 7 10 10M17 7 7 17" />
	{:else if name === 'chevron'}
		<path d="m9 6 6 6-6 6" />
	{:else if name === 'more'}
		<circle cx="5" cy="12" r="1" fill="currentColor" stroke="none" />
		<circle cx="12" cy="12" r="1" fill="currentColor" stroke="none" />
		<circle cx="19" cy="12" r="1" fill="currentColor" stroke="none" />
	{:else if name === 'minimize'}
		<path d="M5 12h14" />
	{:else if name === 'maximize'}
		<rect x="5.5" y="5.5" width="13" height="13" rx="0.5" />
	{:else if name === 'restore'}
		<path d="M8.5 8.5V5.5h10v10h-3" />
		<rect x="5.5" y="8.5" width="10" height="10" rx="0.5" />
	{:else if name === 'font'}
		<path d="M5 18.5 10.6 5.5h2.8L19 18.5M7.4 13h9.2" />
	{:else if name === 'sidebar'}
		<rect x="4" y="5" width="16" height="14" rx="2" />
		<path d="M9.5 5v14" />
	{:else if name === 'copy'}
		<rect x="9" y="9" width="10.5" height="10.5" rx="2" />
		<path d="M15 6.5A2 2 0 0 0 13 4.5H6.5a2 2 0 0 0-2 2V13a2 2 0 0 0 2 2" />
	{:else if name === 'folder'}
		<path
			d="M4.5 7.25A1.75 1.75 0 0 1 6.25 5.5h3.1l1.8 2.25h6.6a1.75 1.75 0 0 1 1.75 1.75v7A1.75 1.75 0 0 1 17.75 18.5H6.25A1.75 1.75 0 0 1 4.5 16.75Z"
		/>
	{:else if name === 'cut'}
		<circle cx="7" cy="17" r="2.5" />
		<circle cx="17" cy="17" r="2.5" />
		<path d="M8.6 15.1 17 4.5M15.4 15.1 7 4.5" />
	{:else if name === 'paste'}
		<path
			d="M9.5 5.5H7.75A1.75 1.75 0 0 0 6 7.25v10.5a1.75 1.75 0 0 0 1.75 1.75h8.5A1.75 1.75 0 0 0 18 17.75V7.25A1.75 1.75 0 0 0 16.25 5.5H14.5"
		/>
		<rect x="9.5" y="3.75" width="5" height="3.5" rx="1.25" />
	{:else if name === 'select-all'}
		<path
			d="M5 8.5v-2a1.5 1.5 0 0 1 1.5-1.5h2M15.5 5h2A1.5 1.5 0 0 1 19 6.5v2M19 15.5v2a1.5 1.5 0 0 1-1.5 1.5h-2M8.5 19h-2A1.5 1.5 0 0 1 5 17.5v-2"
		/>
		<path d="M9 12h6" />
	{:else if name === 'sparkle'}
		<path
			d="M12 4.5c.35 3.4 2.1 5.15 5.5 5.5-3.4.35-5.15 2.1-5.5 5.5-.35-3.4-2.1-5.15-5.5-5.5 3.4-.35 5.15-2.1 5.5-5.5Z"
		/>
		<path
			d="M18 15.5c.16 1.55.95 2.34 2.5 2.5-1.55.16-2.34.95-2.5 2.5-.16-1.55-.95-2.34-2.5-2.5 1.55-.16 2.34-.95 2.5-2.5Z"
		/>
	{/if}
</svg>
