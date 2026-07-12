import { describe, expect, it } from 'vitest';
import type { Guest } from '$lib/api';
import { computeStats, filterSortGuests } from './stats';

function guest(p: Partial<Guest>): Guest {
	return {
		id: p.id ?? 'g',
		name: p.name ?? 'Guest',
		channel: p.channel ?? 'einvite',
		email: p.email ?? null,
		phone: p.phone ?? null,
		max_party_size: p.max_party_size ?? 2,
		rsvp_status: p.rsvp_status ?? 'pending',
		party_size: p.party_size ?? null,
		invite_url: p.invite_url ?? 'http://x/invite/t'
	};
}

const list: Guest[] = [
	guest({ id: '1', name: 'Bianca', channel: 'einvite', rsvp_status: 'attending', party_size: 2 }),
	guest({ id: '2', name: 'aaron', channel: 'print', rsvp_status: 'pending', max_party_size: 1 }),
	guest({ id: '3', name: 'Carl', channel: 'einvite', rsvp_status: 'declined', party_size: 0 }),
	guest({ id: '4', name: 'Dan', channel: 'einvite', rsvp_status: 'attending', party_size: 3, email: 'dan@x.com' })
];

describe('computeStats', () => {
	it('counts channels, statuses, and confirmed headcount', () => {
		const s = computeStats(list);
		expect(s).toMatchObject({
			total: 4,
			einvite: 3,
			print: 1,
			attending: 2,
			pending: 1,
			declined: 1,
			headcount: 5
		});
	});
});

describe('filterSortGuests', () => {
	const base = { search: '', channel: 'all', status: 'all' } as const;

	it('filters by channel and status', () => {
		expect(filterSortGuests(list, { ...base, channel: 'print' }, 'name', 'asc')).toHaveLength(1);
		expect(filterSortGuests(list, { ...base, status: 'attending' }, 'name', 'asc')).toHaveLength(2);
	});

	it('searches name and contact case-insensitively', () => {
		const out = filterSortGuests(list, { ...base, search: 'DAN@X' }, 'name', 'asc');
		expect(out.map((g) => g.id)).toEqual(['4']);
	});

	it('sorts by name ascending/descending, case-insensitive', () => {
		const asc = filterSortGuests(list, base, 'name', 'asc').map((g) => g.name);
		expect(asc).toEqual(['aaron', 'Bianca', 'Carl', 'Dan']);
		const desc = filterSortGuests(list, base, 'name', 'desc').map((g) => g.name);
		expect(desc[0]).toBe('Dan');
	});
});
