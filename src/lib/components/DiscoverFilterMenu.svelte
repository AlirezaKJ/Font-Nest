<script lang="ts" module>
	export type DiscoverFilterOption = {
		value: string;
		label: string;
		description?: string;
	};
</script>

<script lang="ts">
	import { onDestroy, onMount } from 'svelte';

	import Icon from './Icon.svelte';

	let {
		id,
		label,
		value,
		options,
		onChange
	}: {
		id: string;
		label: string;
		value: string;
		options: DiscoverFilterOption[];
		onChange: (value: string) => void;
	} = $props();

	let control = $state<HTMLDivElement>();
	let trigger = $state<HTMLButtonElement>();
	let menu = $state<HTMLDivElement>();
	let open = $state(false);

	let selected = $derived(options.find((option) => option.value === value) ?? options[0]);

	onMount(() => {
		const closeForLayoutChange = () => closeMenu();
		window.addEventListener('resize', closeForLayoutChange);
		window.addEventListener('scroll', closeForLayoutChange, true);
		return () => {
			window.removeEventListener('resize', closeForLayoutChange);
			window.removeEventListener('scroll', closeForLayoutChange, true);
		};
	});

	onDestroy(() => closeMenu());

	function optionButtons() {
		return menu ? Array.from(menu.querySelectorAll<HTMLButtonElement>('[role="option"]')) : [];
	}

	function focusOption(target: 'selected' | 'first' | 'last' = 'selected') {
		const buttons = optionButtons();
		const next =
			target === 'first'
				? buttons[0]
				: target === 'last'
					? buttons.at(-1)
					: (buttons.find((button) => button.dataset.value === value) ?? buttons[0]);
		next?.focus();
	}

	function openMenu(target: 'selected' | 'first' | 'last' = 'selected') {
		if (!control || !menu) return;
		const rect = control.getBoundingClientRect();
		const width = Math.max(196, rect.width);
		const left = Math.max(8, Math.min(rect.left, window.innerWidth - width - 8));
		menu.style.setProperty('--discover-menu-top', `${rect.bottom + 6}px`);
		menu.style.setProperty('--discover-menu-left', `${left}px`);
		menu.style.setProperty('--discover-menu-width', `${width}px`);
		if (!menu.matches(':popover-open')) menu.showPopover();
		open = true;
		requestAnimationFrame(() => focusOption(target));
	}

	function closeMenu(restoreFocus = false) {
		if (menu?.matches(':popover-open')) menu.hidePopover();
		open = false;
		if (restoreFocus) trigger?.focus();
	}

	function toggleMenu() {
		if (open) closeMenu();
		else openMenu();
	}

	function selectOption(nextValue: string) {
		if (nextValue !== value) onChange(nextValue);
		closeMenu(true);
	}

	function handleTriggerKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			openMenu(event.key === 'ArrowDown' ? 'first' : 'last');
		}
	}

	function handleMenuKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			closeMenu(true);
			return;
		}
		if (event.key === 'Tab') {
			closeMenu();
			return;
		}

		const buttons = optionButtons();
		if (!buttons.length) return;
		const currentIndex = buttons.indexOf(document.activeElement as HTMLButtonElement);
		let nextIndex: number | null = null;
		if (event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % buttons.length;
		if (event.key === 'ArrowUp') {
			nextIndex = (currentIndex - 1 + buttons.length) % buttons.length;
		}
		if (event.key === 'Home') nextIndex = 0;
		if (event.key === 'End') nextIndex = buttons.length - 1;
		if (nextIndex === null) return;
		event.preventDefault();
		buttons[nextIndex]?.focus();
	}
</script>

<div bind:this={control} class:active={value !== 'all'} class="filter-control">
	<span id={`${id}-label`}>{label}</span>
	<button
		bind:this={trigger}
		type="button"
		class:open
		aria-haspopup="listbox"
		aria-expanded={open}
		aria-controls={`${id}-menu`}
		aria-labelledby={`${id}-label ${id}-value`}
		onclick={toggleMenu}
		onkeydown={handleTriggerKeydown}
	>
		<strong id={`${id}-value`}>{selected?.label ?? 'All'}</strong>
		<span class="chevron"><Icon name="chevron" size={14} /></span>
	</button>
</div>

<div
	bind:this={menu}
	id={`${id}-menu`}
	class="filter-menu"
	popover="auto"
	role="listbox"
	tabindex="-1"
	aria-labelledby={`${id}-label`}
	onkeydown={handleMenuKeydown}
	ontoggle={(event) => (open = event.newState === 'open')}
>
	{#each options as option (option.value)}
		<button
			type="button"
			role="option"
			class:selected={value === option.value}
			aria-selected={value === option.value}
			data-value={option.value}
			onclick={() => selectOption(option.value)}
		>
			<span>
				<strong>{option.label}</strong>
				{#if option.description}<small>{option.description}</small>{/if}
			</span>
			<span class="check"
				>{#if value === option.value}<Icon name="check" size={14} />{/if}</span
			>
		</button>
	{/each}
</div>

<style>
	.filter-control {
		display: grid;
		min-width: 142px;
		grid-template-columns: auto minmax(0, 1fr);
		align-items: center;
		gap: var(--space-sm);
		padding-left: 10px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-subtle);
		background: var(--color-control);
		font-size: var(--text-micro);
		transition:
			border-color var(--motion-fast),
			background var(--motion-fast);
	}

	.filter-control:hover,
	.filter-control:focus-within {
		border-color: var(--color-subtle);
	}

	.filter-control.active {
		border-color: color-mix(in srgb, var(--color-accent) 58%, var(--color-border));
		background: color-mix(in srgb, var(--color-accent) 6%, var(--color-control));
	}

	.filter-control > button {
		display: flex;
		min-width: 0;
		height: 38px;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-sm);
		padding: 0 9px 0 0;
		border: 0;
		color: var(--color-text);
		background: transparent;
		font-size: var(--text-label);
		text-align: left;
		cursor: pointer;
	}

	.filter-control > button strong {
		overflow: hidden;
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.chevron {
		display: grid;
		flex: none;
		place-items: center;
		transform: rotate(90deg);
		transition: transform var(--motion-fast);
	}

	button.open .chevron {
		transform: rotate(-90deg);
	}

	.filter-menu {
		position: fixed;
		inset: auto;
		top: var(--discover-menu-top);
		left: var(--discover-menu-left);
		z-index: var(--z-dropdown);
		width: var(--discover-menu-width);
		max-height: min(360px, calc(100dvh - var(--discover-menu-top) - 12px));
		margin: 0;
		padding: 4px;
		overflow-y: auto;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-raised);
		box-shadow: var(--shadow-floating);
	}

	.filter-menu::backdrop {
		background: transparent;
	}

	.filter-menu > button {
		display: grid;
		width: 100%;
		min-height: 38px;
		grid-template-columns: minmax(0, 1fr) 18px;
		align-items: center;
		gap: var(--space-sm);
		padding: 7px 8px;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		text-align: left;
		cursor: pointer;
	}

	.filter-menu > button:hover,
	.filter-menu > button:focus-visible {
		color: var(--color-text);
		background: var(--color-hover);
	}

	.filter-menu > button.selected {
		color: var(--color-text);
		background: var(--color-selected);
	}

	.filter-menu button > span:first-child {
		display: grid;
		gap: 2px;
	}

	.filter-menu button strong {
		font-weight: 650;
	}

	.filter-menu button small {
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.check {
		display: grid;
		place-items: center;
	}

	@media (max-width: 760px) {
		.filter-control {
			min-width: 132px;
			flex: 1 1 132px;
		}
	}
</style>
