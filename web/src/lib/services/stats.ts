import type { Guest, InviteChannel, RsvpStatus } from '$lib/api';

/** Aggregate counts for an event's guest list. Pure — computed client-side. */
export interface GuestStats {
	total: number;
	einvite: number;
	print: number;
	attending: number;
	pending: number;
	declined: number;
	/** Total confirmed headcount across attending guests. */
	headcount: number;
}

export function computeStats(guests: Guest[]): GuestStats {
	const stats: GuestStats = {
		total: guests.length,
		einvite: 0,
		print: 0,
		attending: 0,
		pending: 0,
		declined: 0,
		headcount: 0
	};
	for (const g of guests) {
		if (g.channel === 'einvite') stats.einvite++;
		else stats.print++;
		if (g.rsvp_status === 'attending') {
			stats.attending++;
			stats.headcount += g.party_size ?? 0;
		} else if (g.rsvp_status === 'pending') stats.pending++;
		else stats.declined++;
	}
	return stats;
}

export type SortKey = 'name' | 'max_party_size' | 'rsvp_status';
export type SortDir = 'asc' | 'desc';

export interface GuestFilter {
	search: string;
	channel: InviteChannel | 'all';
	status: RsvpStatus | 'all';
}

/** Client-side search + filter + sort over the guest list. Pure. */
export function filterSortGuests(
	guests: Guest[],
	filter: GuestFilter,
	sortKey: SortKey,
	sortDir: SortDir
): Guest[] {
	const q = filter.search.trim().toLowerCase();
	const filtered = guests.filter((g) => {
		if (filter.channel !== 'all' && g.channel !== filter.channel) return false;
		if (filter.status !== 'all' && g.rsvp_status !== filter.status) return false;
		if (q) {
			const hay = `${g.name} ${g.email ?? ''} ${g.phone ?? ''}`.toLowerCase();
			if (!hay.includes(q)) return false;
		}
		return true;
	});

	const dir = sortDir === 'asc' ? 1 : -1;
	return [...filtered].sort((a, b) => {
		let av: string | number;
		let bv: string | number;
		if (sortKey === 'max_party_size') {
			av = a.max_party_size;
			bv = b.max_party_size;
		} else if (sortKey === 'rsvp_status') {
			av = a.rsvp_status;
			bv = b.rsvp_status;
		} else {
			av = a.name.toLowerCase();
			bv = b.name.toLowerCase();
		}
		if (av < bv) return -1 * dir;
		if (av > bv) return 1 * dir;
		return 0;
	});
}
