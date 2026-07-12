import type { CreateGuest, InviteChannel } from '$lib/api';

/**
 * Parse pasted CSV text into guest rows. One guest per line:
 * `name, channel, email, phone, max_party_size`. Channel is normalized
 * ("print" → print, anything else → einvite). Rows without a name are dropped.
 * Pure and dependency-free so it's trivially testable.
 */
export function parseGuestCsv(text: string): CreateGuest[] {
	return text
		.split('\n')
		.map((line) => line.trim())
		.filter(Boolean)
		.map((line) => {
			const [name, channelRaw, email, phone, maxRaw] = line.split(',').map((p) => p.trim());
			const channel: InviteChannel =
				(channelRaw ?? '').toLowerCase() === 'print' ? 'print' : 'einvite';
			const max = Number(maxRaw);
			return {
				name: name ?? '',
				channel,
				email: email || null,
				phone: phone || null,
				max_party_size: Number.isFinite(max) && max >= 1 ? Math.floor(max) : 1
			} satisfies CreateGuest;
		})
		.filter((row) => row.name.length > 0);
}
