type QueueEntry<T> = {
	key: string;
	task: () => Promise<T>;
	resolve: (value: T | PromiseLike<T>) => void;
	reject: (reason?: unknown) => void;
};

export class KeyedTaskQueue<T> {
	readonly #concurrency: number;
	readonly #entries: Array<QueueEntry<T>> = [];
	readonly #promises = new Map<string, Promise<T>>();
	#active = 0;

	constructor(concurrency: number) {
		if (!Number.isInteger(concurrency) || concurrency < 1) {
			throw new RangeError('Queue concurrency must be a positive integer.');
		}
		this.#concurrency = concurrency;
	}

	enqueue(key: string, task: () => Promise<T>): Promise<T> {
		const existing = this.#promises.get(key);
		if (existing) return existing;

		const promise = new Promise<T>((resolve, reject) => {
			this.#entries.push({ key, task, resolve, reject });
		});
		this.#promises.set(key, promise);
		this.#drain();
		return promise;
	}

	#drain() {
		while (this.#active < this.#concurrency && this.#entries.length) {
			const entry = this.#entries.shift();
			if (!entry) return;
			this.#active += 1;
			void Promise.resolve()
				.then(entry.task)
				.then(entry.resolve, entry.reject)
				.finally(() => {
					this.#active -= 1;
					this.#promises.delete(entry.key);
					this.#drain();
				});
		}
	}
}

export function pickPreviewEvictionCandidate(
	oldestFirstIds: readonly string[],
	visibleIds: ReadonlySet<string>,
	protectedId: string | null
): string | null {
	return (
		oldestFirstIds.find((familyId) => familyId !== protectedId && !visibleIds.has(familyId)) ??
		null
	);
}
