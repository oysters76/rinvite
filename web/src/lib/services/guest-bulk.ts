// Bulk guest operations the backend has no single endpoint for. These orchestrate
// the per-guest API (`$lib/api`) with bounded concurrency and aggregate partial
// failures — keeping the "missing" features out of the components.

import { ApiError, guests as guestsApi, invites } from '$lib/api';
import type { BatchSendReport, Guest, InviteChannel, SendResult } from '$lib/api';

const CONCURRENCY = 4;

/** Run `fn` over `items` with at most `limit` in flight; never rejects. */
async function mapSettled<T, R>(
	items: T[],
	limit: number,
	fn: (item: T) => Promise<R>
): Promise<PromiseSettledResult<R>[]> {
	const results: PromiseSettledResult<R>[] = new Array(items.length);
	let cursor = 0;
	async function worker() {
		while (cursor < items.length) {
			const i = cursor++;
			try {
				results[i] = { status: 'fulfilled', value: await fn(items[i]) };
			} catch (reason) {
				results[i] = { status: 'rejected', reason };
			}
		}
	}
	await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
	return results;
}

function errText(reason: unknown): string {
	return reason instanceof ApiError ? reason.message : 'Request failed';
}

export interface BulkOutcome {
	ok: number;
	failed: number;
}

/** Switch the channel of many guests. */
export async function moveMany(
	eventId: string,
	ids: string[],
	channel: InviteChannel
): Promise<BulkOutcome> {
	const settled = await mapSettled(ids, CONCURRENCY, (id) =>
		guestsApi.update(eventId, id, { channel })
	);
	return tally(settled);
}

/** Delete many guests. */
export async function deleteMany(eventId: string, ids: string[]): Promise<BulkOutcome> {
	const settled = await mapSettled(ids, CONCURRENCY, (id) => guestsApi.remove(eventId, id));
	return tally(settled);
}

/**
 * Send e-invites to a specific set of guests (the backend only exposes
 * "send all"). Only e-invite-channel guests are contacted; returns a report in
 * the same shape as the backend's bulk send.
 */
export async function sendSelected(eventId: string, selected: Guest[]): Promise<BatchSendReport> {
	const targets = selected.filter((g) => g.channel === 'einvite');
	const settled = await mapSettled(targets, CONCURRENCY, (g) => invites.sendGuest(eventId, g.id));
	const results: SendResult[] = targets.map((g, i) => {
		const r = settled[i];
		return r.status === 'fulfilled'
			? { guest_id: g.id, guest_name: g.name, status: 'sent', detail: null }
			: { guest_id: g.id, guest_name: g.name, status: 'failed', detail: errText(r.reason) };
	});
	return {
		total: results.length,
		sent: results.filter((r) => r.status === 'sent').length,
		failed: results.filter((r) => r.status === 'failed').length,
		results
	};
}

function tally(settled: PromiseSettledResult<unknown>[]): BulkOutcome {
	return {
		ok: settled.filter((r) => r.status === 'fulfilled').length,
		failed: settled.filter((r) => r.status === 'rejected').length
	};
}
