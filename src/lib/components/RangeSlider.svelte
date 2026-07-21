<script lang="ts">
	// One slider for the whole app. The native control cannot paint the part of the track
	// behind the thumb, so the filled portion is a gradient stop driven by --range-fill and
	// the thumb is drawn from scratch. Everything else (keyboard, drag, step snapping) stays
	// the browser's job.
	let {
		label,
		value,
		min,
		max,
		step = 1,
		display,
		valueText,
		labelWidth = 'auto',
		valueWidth = '48px',
		trackWidth = '96px',
		disabled = false,
		onChange
	}: {
		label: string;
		value: number;
		min: number;
		max: number;
		step?: number;
		display: string;
		valueText?: string;
		labelWidth?: string;
		valueWidth?: string;
		trackWidth?: string;
		disabled?: boolean;
		onChange: (value: number) => void;
	} = $props();

	let fill = $derived.by(() => {
		const span = max - min;
		if (span <= 0) return 0;
		const ratio = (Math.min(Math.max(value, min), max) - min) / span;
		return Math.round(ratio * 1000) / 10;
	});
</script>

<label
	class="range-field"
	class:disabled
	style={`--range-fill: ${fill}%; --range-label-width: ${labelWidth}; --range-value-width: ${valueWidth}; --range-track-width: ${trackWidth};`}
>
	<span class="range-label">{label}</span>
	<input
		type="range"
		{min}
		{max}
		{step}
		{value}
		{disabled}
		aria-label={label}
		aria-valuetext={valueText}
		oninput={(event) => onChange(Number(event.currentTarget.value))}
	/>
	<output class="range-value">{display}</output>
</label>

<style>
	.range-field {
		display: grid;
		flex: none;
		grid-template-columns:
			var(--range-label-width) var(--range-track-width)
			var(--range-value-width);
		align-items: center;
		gap: var(--space-sm);
		color: var(--color-subtle);
		font-size: var(--text-micro);
	}

	.range-field.disabled {
		opacity: 0.55;
	}

	.range-label {
		white-space: nowrap;
	}

	.range-value {
		color: var(--color-muted);
		font-size: var(--text-label);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	input[type='range'] {
		width: 100%;
		height: 18px;
		margin: 0;
		padding: 0;
		border: 0;
		background: transparent;
		appearance: none;
		cursor: pointer;
	}

	input[type='range']:disabled {
		cursor: not-allowed;
	}

	/* WebKit and Chromium (the Tauri web views on every platform we ship). */
	input[type='range']::-webkit-slider-runnable-track {
		height: 6px;
		border-radius: 999px;
		background:
			linear-gradient(var(--color-accent), var(--color-accent)) 0 / var(--range-fill) 100%
				no-repeat,
			var(--color-border);
		transition: background-color 140ms ease;
	}

	input[type='range']::-webkit-slider-thumb {
		width: 14px;
		height: 14px;
		margin-top: -4px;
		border: 2px solid var(--color-control);
		border-radius: 50%;
		background: var(--color-accent);
		box-shadow: 0 1px 3px rgb(0 0 0 / 26%);
		appearance: none;
		transition:
			transform 130ms ease,
			box-shadow 130ms ease,
			background-color 130ms ease;
	}

	input[type='range']:hover::-webkit-slider-thumb {
		background: var(--color-accent-hover);
		transform: scale(1.14);
	}

	input[type='range']:active::-webkit-slider-thumb {
		transform: scale(1.04);
		box-shadow: 0 0 0 5px color-mix(in srgb, var(--color-accent) 22%, transparent);
	}

	input[type='range']:disabled::-webkit-slider-thumb {
		background: var(--color-subtle);
		box-shadow: none;
		transform: none;
	}

	/* Gecko, for anyone running the dev server in Firefox. */
	input[type='range']::-moz-range-track {
		height: 6px;
		border-radius: 999px;
		background: var(--color-border);
	}

	input[type='range']::-moz-range-progress {
		height: 6px;
		border-radius: 999px;
		background: var(--color-accent);
	}

	input[type='range']::-moz-range-thumb {
		width: 14px;
		height: 14px;
		border: 2px solid var(--color-control);
		border-radius: 50%;
		background: var(--color-accent);
		box-shadow: 0 1px 3px rgb(0 0 0 / 26%);
		transition:
			transform 130ms ease,
			box-shadow 130ms ease,
			background-color 130ms ease;
	}

	input[type='range']:hover::-moz-range-thumb {
		background: var(--color-accent-hover);
		transform: scale(1.14);
	}

	@media (prefers-reduced-motion: reduce) {
		input[type='range']::-webkit-slider-thumb,
		input[type='range']::-moz-range-thumb {
			transition: none;
		}

		input[type='range']:hover::-webkit-slider-thumb,
		input[type='range']:active::-webkit-slider-thumb,
		input[type='range']:hover::-moz-range-thumb {
			transform: none;
		}
	}

	@media (max-width: 1080px) {
		.range-field {
			grid-template-columns:
				var(--range-label-width) minmax(80px, 1fr)
				var(--range-value-width);
			width: 100%;
		}
	}
</style>
