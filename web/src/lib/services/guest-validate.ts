import type { CreateGuest } from '$lib/api';

// Client-side mirror of the backend's `validate_guest` (src/domain/validation.rs)
// so the import preview can flag bad rows before the request. The backend stays
// the source of truth and re-validates atomically; this is purely for UX.

const NAME_MAX_LEN = 200;
const EMAIL_MIN_LEN = 3;
const EMAIL_MAX_LEN = 254;
const PHONE_MAX_LEN = 32;

/** Structural email check matching the backend: one `@`, non-empty local part, dotted domain. */
function emailError(email: string): string | null {
	const e = email.trim();
	if (e.length < EMAIL_MIN_LEN || e.length > EMAIL_MAX_LEN) return 'email length is out of range';
	const parts = e.split('@');
	if (parts.length !== 2) return "email must contain exactly one '@'";
	const [local, domain] = parts;
	const domainHasDot = domain.includes('.') && !domain.startsWith('.') && !domain.endsWith('.');
	if (!local || !domain || !domainHasDot) return 'email is not a valid address';
	return null;
}

/**
 * Return a list of human-readable problems with a guest row (empty = valid).
 * Mirrors the backend rules: a name (≤200 chars), a party size ≥ 1, and
 * well-formed contact details when present.
 */
export function validateGuestRow(row: CreateGuest): string[] {
	const errors: string[] = [];

	const name = (row.name ?? '').trim();
	if (name.length === 0) errors.push('name must not be empty');
	else if (name.length > NAME_MAX_LEN) errors.push(`name must be at most ${NAME_MAX_LEN} characters`);

	if (!Number.isInteger(row.max_party_size) || row.max_party_size < 1)
		errors.push('max party size must be a whole number of at least 1');

	if (row.email) {
		const e = emailError(row.email);
		if (e) errors.push(e);
	}

	if (row.phone) {
		const p = row.phone.trim();
		if (p.length === 0 || p.length > PHONE_MAX_LEN) errors.push('phone must be 1–32 characters');
	}

	return errors;
}
