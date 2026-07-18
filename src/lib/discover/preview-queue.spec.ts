import { describe, expect, it } from 'vitest';

import { KeyedTaskQueue, pickPreviewEvictionCandidate } from './preview-queue';

describe('KeyedTaskQueue', () => {
	it('deduplicates keys and caps concurrent preview work', async () => {
		let active = 0;
		let peak = 0;
		const releases: Array<() => void> = [];
		const queue = new KeyedTaskQueue<string>(2);
		const task = (value: string) => async () => {
			active += 1;
			peak = Math.max(peak, active);
			await new Promise<void>((resolve) => releases.push(resolve));
			active -= 1;
			return value;
		};

		const first = queue.enqueue('inter', task('Inter'));
		const duplicate = queue.enqueue('inter', task('Duplicate'));
		const second = queue.enqueue('fraunces', task('Fraunces'));
		const third = queue.enqueue('instrument-sans', task('Instrument Sans'));

		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(first).toBe(duplicate);
		expect(active).toBe(2);
		expect(peak).toBe(2);
		expect(releases).toHaveLength(2);

		releases.shift()?.();
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(active).toBe(2);
		expect(peak).toBe(2);

		while (releases.length) releases.shift()?.();
		await expect(Promise.all([first, second, third])).resolves.toEqual([
			'Inter',
			'Fraunces',
			'Instrument Sans'
		]);
	});
});

describe('pickPreviewEvictionCandidate', () => {
	it('evicts the oldest off-screen face and protects the expanded family', () => {
		expect(
			pickPreviewEvictionCandidate(
				['inter', 'fraunces', 'instrument-sans'],
				new Set(['fraunces']),
				'inter'
			)
		).toBe('instrument-sans');
	});
});
