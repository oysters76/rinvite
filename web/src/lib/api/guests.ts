import { del, get, patch, post } from './client';
import type { CreateGuest, Guest, UpdateGuest } from './types';

export const guests = {
	list: (eventId: string) => get<Guest[]>(`/events/${eventId}/guests`),
	create: (eventId: string, data: CreateGuest) =>
		post<Guest>(`/events/${eventId}/guests`, data),
	get: (eventId: string, guestId: string) =>
		get<Guest>(`/events/${eventId}/guests/${guestId}`),
	update: (eventId: string, guestId: string, data: UpdateGuest) =>
		patch<Guest>(`/events/${eventId}/guests/${guestId}`, data),
	remove: (eventId: string, guestId: string) =>
		del(`/events/${eventId}/guests/${guestId}`)
};
