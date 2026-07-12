import { auth as authApi } from '$lib/api';
import type { User } from '$lib/api';
import { clearToken, isAuthenticated } from './session';

/** Reactive current-user state for the app shell. */
class AuthStore {
	user = $state<User | null>(null);

	get isAuthenticated(): boolean {
		return isAuthenticated();
	}

	/** Load the authenticated user (called by the app guard). */
	async load(): Promise<User> {
		this.user = await authApi.me();
		return this.user;
	}

	logout(): void {
		clearToken();
		this.user = null;
	}
}

export const authStore = new AuthStore();
