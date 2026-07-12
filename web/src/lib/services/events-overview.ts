import { events as eventsApi, guests as guestsApi } from '$lib/api';
import type { Event, Guest } from '$lib/api';

/** An event enriched with the per-channel guest counts the cards display. */
export interface EventOverview extends Event {
	guestCount: number;
	einviteCount: number;
	printCount: number;
}

/**
 * Load all events with their guest counts. The backend has no counts aggregate,
 * so we fetch guests per event in parallel (fine at personal-organizer scale; a
 * backend aggregate is a future optimization).
 */
export async function loadEventsOverview(): Promise<EventOverview[]> {
	const events = await eventsApi.list();
	return Promise.all(
		events.map(async (ev) => {
			let guests: Guest[] = [];
			try {
				guests = await guestsApi.list(ev.id);
			} catch {
				guests = [];
			}
			return {
				...ev,
				guestCount: guests.length,
				einviteCount: guests.filter((g) => g.channel === 'einvite').length,
				printCount: guests.filter((g) => g.channel === 'print').length
			};
		})
	);
}
