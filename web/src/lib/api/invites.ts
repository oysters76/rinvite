import { downloadBlob, post } from './client';
import type { BatchSendReport, SendResponse } from './types';

export const invites = {
	/** Download one guest's printable PDF (blob + filename). */
	guestPdf: (eventId: string, guestId: string) =>
		downloadBlob(`/events/${eventId}/guests/${guestId}/invite.pdf`),
	/** Deliver one guest's e-invite via the configured sender. */
	sendGuest: (eventId: string, guestId: string) =>
		post<SendResponse>(`/events/${eventId}/guests/${guestId}/send`),
	/** Download the merged PDF of every print-channel guest. */
	printBatch: (eventId: string) => downloadBlob(`/events/${eventId}/invites/print.pdf`),
	/** Sequentially send every e-invite; returns a per-guest report. */
	sendBatch: (eventId: string) => post<BatchSendReport>(`/events/${eventId}/invites/send`)
};
