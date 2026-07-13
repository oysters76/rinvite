import { setToken } from '../stores/session';
import { get, post } from './client';
import type { AuthResponse, User } from './types';

export const auth = {
	/**
	 * Create an account. No token is issued — the backend emails a verification
	 * link and the user must verify before logging in.
	 */
	async signup(email: string, password: string): Promise<void> {
		await post('/auth/signup', { email, password }, false);
	},
	/** Log in and store the returned token. */
	async login(email: string, password: string): Promise<void> {
		const res = await post<AuthResponse>('/auth/login', { email, password }, false);
		setToken(res.token);
	},
	/** Confirm an email-verification token. */
	verify: (token: string) => post<void>('/auth/verify', { token }, false),
	/** Ask for a fresh verification email. Always resolves (never reveals state). */
	resendVerification: (email: string) =>
		post<void>('/auth/resend-verification', { email }, false),
	/** The currently authenticated user. */
	me: () => get<User>('/auth/me')
};
