import { del, get, patch, post } from './client';
import type { CreateEvent, Event, UpdateEvent } from './types';

export const events = {
	list: () => get<Event[]>('/events'),
	create: (data: CreateEvent) => post<Event>('/events', data),
	get: (id: string) => get<Event>(`/events/${id}`),
	update: (id: string, data: UpdateEvent) => patch<Event>(`/events/${id}`, data),
	remove: (id: string) => del(`/events/${id}`)
};
