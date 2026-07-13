// TypeScript mirrors of the backend JSON DTOs. Field names are kept snake_case
// to match the API exactly — no transformation layer.

export interface AuthResponse {
	token: string;
}

export type Plan = 'free' | 'pro' | 'max';

export interface User {
	id: string;
	email: string;
	plan: Plan;
	email_verified: boolean;
}

/** Public client configuration served by `GET /config`. */
export interface AppConfig {
	contact_email: string;
}

export interface Event {
	id: string;
	bride_name: string;
	bride_family_name: string;
	groom_name: string;
	groom_family_name: string;
	/** ISO date, YYYY-MM-DD */
	event_date: string;
	/** HH:MM:SS */
	start_time: string;
	end_time: string;
	hall_name: string;
	venue_name: string;
	rsvp_by: string;
}

export type CreateEvent = Omit<Event, 'id'>;

/** Partial update — only the provided fields change. */
export type UpdateEvent = Partial<CreateEvent>;

export type InviteChannel = 'print' | 'einvite';
export type RsvpStatus = 'pending' | 'attending' | 'declined';

export interface Guest {
	id: string;
	name: string;
	channel: InviteChannel;
	email: string | null;
	phone: string | null;
	max_party_size: number;
	rsvp_status: RsvpStatus;
	party_size: number | null;
	/** Shareable e-invite link for this guest. */
	invite_url: string;
}

export interface CreateGuest {
	name: string;
	channel: InviteChannel;
	email?: string | null;
	phone?: string | null;
	max_party_size: number;
}

export type UpdateGuest = Partial<CreateGuest>;

export interface SendResponse {
	sent: boolean;
	invite_url: string;
}

export type SendStatus = 'sent' | 'failed';

export interface SendResult {
	guest_id: string;
	guest_name: string;
	status: SendStatus;
	detail: string | null;
}

export interface BatchSendReport {
	total: number;
	sent: number;
	failed: number;
	results: SendResult[];
}
