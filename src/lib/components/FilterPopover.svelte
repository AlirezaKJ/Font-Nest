<script lang="ts" module>
	import type { DiscoverFilterOption } from './DiscoverFilterMenu.svelte';

	export type FilterGroup = {
		key: string;
		label: string;
		value: string;
		options: DiscoverFilterOption[];
	};
</script>

<script lang="ts">
	import { onDestroy, onMount } from 'svelte';

	import Icon from './Icon.svelte';

	let {
		id,
		groups,
		onChange,
		onClear
	}: {
		id: string;
		groups: FilterGroup[];
		onChange: (key: string, value: string) => void;
		onClear: () => void;
	} = $props();

	let trigger = $state<HTMLButtonElement>();
	let panel = $state<HTMLDivElement>();
	let open = $state(false);

	let activeCount = $derived(groups.filter((group) => group.value !== 'all').length);

	onMount(() => {
		const closeForResize = () => closeMenu();
		// The panel is anchored to the trigger's viewport position, so scrolling the page
		// out from under it has to close it. Scrolling the panel's own option list must
		// not: this listener is on the capture phase and would otherwise see those events.
		const closeForScroll = (event: Event) => {
			const target = event.target as Node | null;
			if (target && panel && (target === panel || panel.contains(target))) return;
			closeMenu();
		};
		window.addEventListener('resize', closeForResize);
		window.addEventListener('scroll', closeForScroll, true);
		return () => {
			window.removeEventListener('resize', closeForResize);
			window.removeEventListener('scroll', closeForScroll, true);
		};
	});

	onDestroy(() => closeMenu());

	function optionButtons() {
		return panel
			? Array.from(panel.querySelectorAll<HTMLButtonElement>('[role="menuitemradio"]'))
			: [];
	}

	function focusOption(target: 'selected' | 'first' | 'last' = 'selected') {
		const buttons = optionButtons();
		const next =
			target === 'first'
				? buttons[0]
				: target === 'last'
					? buttons.at(-1)
					: (buttons.find((button) => button.dataset.checked === 'true') ?? buttons[0]);
		next?.focus();
	}

	function openMenu(target: 'selected' | 'first' | 'last' = 'selected') {
		if (!trigger || !panel) return;
		const rect = trigger.getBoundingClientRect();
		const width = Math.max(268, rect.width);
		const left = Math.max(8, Math.min(rect.left, window.innerWidth - width - 8));
		panel.style.setProperty('--filter-panel-top', `${rect.bottom + 6}px`);
		panel.style.setProperty('--filter-panel-left', `${left}px`);
		panel.style.setProperty('--filter-panel-width', `${width}px`);
		if (!panel.matches(':popover-open')) panel.showPopover();
		open = true;
		requestAnimationFrame(() => focusOption(target));
	}

	function closeMenu(restoreFocus = false) {
		if (panel?.matches(':popover-open')) panel.hidePopover();
		open = false;
		if (restoreFocus) trigger?.focus();
	}

	function toggleMenu() {
		if (open) closeMenu();
		else openMenu();
	}

	// The panel stays open after a choice: setting several filters in one visit is the
	// common case, and reopening between each one is the friction this control removes.
	function selectOption(group: FilterGroup, value: string) {
		if (value !== group.value) onChange(group.key, value);
	}

	function handleTriggerKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			openMenu(event.key === 'ArrowDown' ? 'first' : 'last');
		}
	}

	function handlePanelKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			closeMenu(true);
			return;
		}

		const buttons = optionButtons();
		if (!buttons.length) return;
		const currentIndex = buttons.indexOf(document.activeElement as HTMLButtonElement);
		let nextIndex: number | null = null;
		if (event.key === 'ArrowDown') nextIndex = (currentIndex + 1) % buttons.length;
		if (event.key === 'ArrowUp')
			nextIndex = (currentIndex - 1 + buttons.length) % buttons.length;
		if (event.key === 'Home') nextIndex = 0;
		if (event.key === 'End') nextIndex = buttons.length - 1;
		if (nextIndex === null) return;
		event.preventDefault();
		buttons[nextIndex]?.focus();
	}
</script>

<button
	bind:this={trigger}
	type="button"
	class:active={activeCount > 0}
	class:open
	class="filter-trigger"
	aria-haspopup="menu"
	aria-expanded={open}
	aria-controls={`${id}-panel`}
	onclick={toggleMenu}
	onkeydown={handleTriggerKeydown}
>
	<Icon name="filter" size={15} />
	<span>Filters</span>
	{#if activeCount}
		<span class="count" aria-label={`${activeCount} active`}>{activeCount}</span>
	{/if}
	<span class="chevron"><Icon name="chevron" size={14} /></span>
</button>

<div
	bind:this={panel}
	id={`${id}-panel`}
	class="filter-panel"
	popover="auto"
	role="menu"
	tabindex="-1"
	aria-label="Filters"
	onkeydown={handlePanelKeydown}
	ontoggle={(event) => (open = event.newState === 'open')}
>
	{#each groups as group (group.key)}
		<div class="filter-group" role="group" aria-label={group.label}>
			<p class="group-label">{group.label}</p>
			{#each group.options as option (option.value)}
				<button
					type="button"
					role="menuitemradio"
					class:selected={group.value === option.value}
					aria-checked={group.value === option.value}
					data-checked={group.value === option.value}
					onclick={() => selectOption(group, option.value)}
				>
					<span>
						<strong>{option.label}</strong>
						{#if option.description}<small>{option.description}</small>{/if}
					</span>
					<span class="check"
						>{#if group.value === option.value}<Icon
								name="check"
								size={14}
							/>{/if}</span
					>
				</button>
			{/each}
		</div>
	{/each}

	<div class="panel-footer">
		<button type="button" disabled={activeCount === 0} onclick={onClear}>Clear filters</button>
		<button type="button" onclick={() => closeMenu(true)}>Done</button>
	</div>
</div>

<style>
	.filter-trigger {
		display: inline-flex;
		height: 38px;
		flex: none;
		align-items: center;
		gap: var(--space-sm);
		padding: 0 9px 0 10px;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-control);
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
		transition:
			border-color var(--motion-fast),
			background var(--motion-fast);
	}

	.filter-trigger:hover {
		border-color: var(--color-subtle);
	}

	.filter-trigger.active {
		border-color: color-mix(in srgb, var(--color-accent) 58%, var(--color-border));
		background: color-mix(in srgb, var(--color-accent) 6%, var(--color-control));
	}

	.count {
		display: grid;
		min-width: 18px;
		height: 18px;
		place-items: center;
		padding: 0 5px;
		border-radius: var(--radius-shell);
		color: var(--color-accent-ink);
		background: var(--color-accent);
		font-size: var(--text-micro);
		font-variant-numeric: tabular-nums;
	}

	.chevron {
		display: grid;
		flex: none;
		place-items: center;
		color: var(--color-subtle);
		transform: rotate(90deg);
		transition: transform var(--motion-fast);
	}

	.filter-trigger.open .chevron {
		transform: rotate(-90deg);
	}

	.filter-panel {
		position: fixed;
		inset: auto;
		top: var(--filter-panel-top);
		left: var(--filter-panel-left);
		z-index: var(--z-dropdown);
		width: var(--filter-panel-width);
		max-height: min(440px, calc(100dvh - var(--filter-panel-top) - 12px));
		margin: 0;
		padding: 4px;
		overflow-y: auto;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text);
		background: var(--color-raised);
		box-shadow: var(--shadow-floating);
		overscroll-behavior: contain;
	}

	.filter-panel::backdrop {
		background: transparent;
	}

	.filter-group + .filter-group {
		margin-top: 2px;
		padding-top: 4px;
		border-top: 1px solid var(--color-border);
	}

	.group-label {
		margin: 0;
		padding: 7px 8px 4px;
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.filter-group button {
		display: grid;
		width: 100%;
		min-height: 32px;
		grid-template-columns: minmax(0, 1fr) 18px;
		align-items: center;
		gap: var(--space-sm);
		padding: 5px 8px;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		text-align: left;
		cursor: pointer;
	}

	.filter-group button:hover,
	.filter-group button:focus-visible {
		color: var(--color-text);
		background: var(--color-hover);
	}

	.filter-group button.selected {
		color: var(--color-text);
		background: var(--color-selected);
	}

	.filter-group button > span:first-child {
		display: grid;
		gap: 2px;
	}

	.filter-group button strong {
		overflow: hidden;
		font-weight: 650;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.filter-group button small {
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.check {
		display: grid;
		place-items: center;
	}

	.panel-footer {
		position: sticky;
		bottom: -4px;
		display: flex;
		justify-content: space-between;
		gap: var(--space-sm);
		margin-top: 2px;
		padding: 5px 4px;
		border-top: 1px solid var(--color-border);
		background: var(--color-raised);
	}

	.panel-footer button {
		height: 30px;
		padding: 0 8px;
		border: 0;
		border-radius: var(--radius-sm);
		color: var(--color-muted);
		background: transparent;
		font-size: var(--text-label);
		font-weight: 650;
		cursor: pointer;
	}

	.panel-footer button:hover:not(:disabled) {
		color: var(--color-text);
		background: var(--color-hover);
	}

	.panel-footer button:disabled {
		opacity: 0.45;
		cursor: default;
	}
</style>
