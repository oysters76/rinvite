import { post } from './client';

export const billing = {
	/** Ask the app owner to upgrade the current user's plan (owner is emailed). */
	requestUpgrade: () => post<void>('/billing/upgrade-request')
};
