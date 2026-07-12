import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Guest } from '$lib/api';

// Mock the API layer so the orchestration is tested in isolation.
const sendGuest = vi.fn();
const update = vi.fn();
const remove = vi.fn();
vi.mock('$lib/api', () => ({
	ApiError: class ApiError extends Error {
		status: number;
		constructor(status: number, message: string) {
			super(message);
			this.status = status;
		}
	},
	guests: { update: (...a: unknown[]) => update(...a), remove: (...a: unknown[]) => remove(...a) },
	invites: { sendGuest: (...a: unknown[]) => sendGuest(...a) }
}));

import { deleteMany, moveMany, sendSelected } from './guest-bulk';

function guest(id: string, channel: Guest['channel'] = 'einvite'): Guest {
	return {
		id,
		name: `Guest ${id}`,
		channel,
		email: null,
		phone: null,
		max_party_size: 2,
		rsvp_status: 'pending',
		party_size: null,
		invite_url: 'http://x'
	};
}

beforeEach(() => {
	sendGuest.mockReset();
	update.mockReset();
	remove.mockReset();
});

describe('sendSelected', () => {
	it('only sends e-invite guests and aggregates a report with failures', async () => {
		const selected = [guest('1'), guest('2'), guest('3', 'print')];
		sendGuest.mockResolvedValueOnce({ sent: true }); // g1 ok
		sendGuest.mockRejectedValueOnce(new Error('smtp down')); // g2 fails
		const report = await sendSelected('e1', selected);
		expect(sendGuest).toHaveBeenCalledTimes(2); // print guest skipped
		expect(report.total).toBe(2);
		expect(report.sent).toBe(1);
		expect(report.failed).toBe(1);
		expect(report.results.find((r) => r.guest_id === '2')?.status).toBe('failed');
	});
});

describe('moveMany / deleteMany', () => {
	it('moveMany switches channel for each id and tallies', async () => {
		update.mockResolvedValue(undefined);
		const out = await moveMany('e1', ['1', '2'], 'print');
		expect(update).toHaveBeenCalledWith('e1', '1', { channel: 'print' });
		expect(out).toEqual({ ok: 2, failed: 0 });
	});

	it('deleteMany reports partial failure', async () => {
		remove.mockResolvedValueOnce(undefined);
		remove.mockRejectedValueOnce(new Error('nope'));
		const out = await deleteMany('e1', ['1', '2']);
		expect(out).toEqual({ ok: 1, failed: 1 });
	});
});
