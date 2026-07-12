import { redirect } from '@sveltejs/kit';
import { auth } from '$lib/api';
import { isAuthenticated } from '$lib/stores/session';
import { authStore } from '$lib/stores/auth.svelte';
import type { LayoutLoad } from './$types';

/** Guard every dashboard route: require a token and a valid session. */
export const load: LayoutLoad = async () => {
	if (!isAuthenticated()) throw redirect(307, '/login');
	try {
		const user = await auth.me();
		authStore.user = user;
		return { user };
	} catch {
		throw redirect(307, '/login');
	}
};
