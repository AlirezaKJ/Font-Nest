<script lang="ts" module>
	export type IconName =
		| 'alert'
		| 'check'
		| 'chevron'
		| 'close'
		| 'duplicates'
		| 'filter'
		| 'font'
		| 'library'
		| 'moon'
		| 'monitor'
		| 'refresh'
		| 'search'
		| 'settings'
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
	{:else if name === 'duplicates'}
		<rect x="4.5" y="7.5" width="12" height="12" rx="2" />
		<path
			d="M8 7.5V6.25A1.75 1.75 0 0 1 9.75 4.5h8A1.75 1.75 0 0 1 19.5 6.25v8A1.75 1.75 0 0 1 17.75 16H16.5"
		/>
	{:else if name === 'settings'}
		<circle cx="12" cy="12" r="3" />
		<path
			d="M12 4.5v2M12 17.5v2M19.5 12h-2M6.5 12h-2M17.3 6.7l-1.4 1.4M8.1 15.9l-1.4 1.4M17.3 17.3l-1.4-1.4M8.1 8.1 6.7 6.7"
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
	{:else if name === 'font'}
		<path d="M5 18.5 10.6 5.5h2.8L19 18.5M7.4 13h9.2" />
	{/if}
</svg>
