import { setToken } from '../stores/session';
import { get, post } from './client';
import type { AuthResponse, User } from './types';

export const auth = {
	/** Create an account and store the returned token. */
	async signup(email: string, password: string): Promise<void> {
		const res = await post<AuthResponse>('/auth/signup', { email, password }, false);
		setToken(res.token);
	},
	/** Log in and store the returned token. */
	async login(email: string, password: string): Promise<void> {
		const res = await post<AuthResponse>('/auth/login', { email, password }, false);
		setToken(res.token);
	},
	/** The currently authenticated user. */
	me: () => get<User>('/auth/me')
};
