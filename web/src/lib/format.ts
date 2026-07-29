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

/**
 * Progressively format a Sri Lankan phone number as the user types, to
 * "+94 XX XXX XXXX". Accepts anything (loose digits, a pasted "+94 …", or a
 * local "07…" number) and keeps only the 9 national digits after the +94
 * country code, so the user can just type digits.
 */
export function formatLkPhone(raw: string): string {
	let digits = raw.replace(/\D/g, '');
	// Drop the country code if it's already there (e.g. re-formatting "+94 …").
	if (digits.startsWith('94')) digits = digits.slice(2);
	// Local trunk prefix (e.g. 071…) — the leading 0 isn't part of the +94 form.
	if (digits.startsWith('0')) digits = digits.slice(1);
	digits = digits.slice(0, 9);
	if (!digits) return '';
	const groups = [digits.slice(0, 2), digits.slice(2, 5), digits.slice(5, 9)].filter(Boolean);
	return `+94 ${groups.join(' ')}`;
}

/** True once a phone holds a full "+94 XX XXX XXXX" (9 national digits). */
export const isCompleteLkPhone = (p: string): boolean => /^\+94 \d{2} \d{3} \d{4}$/.test(p);
