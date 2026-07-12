import type { Event } from '$lib/api';

/** "September 25, 2026" from an ISO date. */
export function fmtDate(iso: string): string {
	if (!iso) return '';
	const d = new Date(`${iso}T00:00:00`);
	if (Number.isNaN(d.getTime())) return iso;
	return d.toLocaleDateString(undefined, { month: 'long', day: 'numeric', year: 'numeric' });
}

/** "10:00 AM" from "HH:MM[:SS]". */
export function fmtTime(t: string): string {
	if (!t) return '';
	const [h, m] = t.split(':').map(Number);
	const ap = h < 12 ? 'AM' : 'PM';
	const h12 = h % 12 === 0 ? 12 : h % 12;
	return `${h12}:${String(m).padStart(2, '0')} ${ap}`;
}

export const coupleTitle = (e: Pick<Event, 'bride_name' | 'groom_name'>): string =>
	`${e.bride_name} & ${e.groom_name}`;
