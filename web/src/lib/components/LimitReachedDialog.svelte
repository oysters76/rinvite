<script lang="ts">
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Button } from '$lib/components/ui/button';
	import { ApiError, billing } from '$lib/api';
	import type { Plan } from '$lib/api';
	import { toast } from 'svelte-sonner';

	let {
		open = $bindable(false),
		plan = 'free',
		contactEmail = '',
		message = ''
	}: {
		open?: boolean;
		plan?: Plan;
		contactEmail?: string;
		/** Optional server-supplied detail, e.g. "your plan allows 1 event". */
		message?: string;
	} = $props();

	const planLabel = $derived(plan.charAt(0).toUpperCase() + plan.slice(1));
	let requesting = $state(false);

	async function requestUpgrade() {
		requesting = true;
		try {
			await billing.requestUpgrade();
			toast.success('Upgrade request sent — we’ll be in touch shortly.');
			open = false;
		} catch (err) {
			toast.error(err instanceof ApiError ? err.message : 'Could not send request');
		} finally {
			requesting = false;
		}
	}
</script>

<AlertDialog.Root bind:open>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>You’ve reached your {planLabel} plan limit</AlertDialog.Title>
			<AlertDialog.Description>
				{message || 'This action exceeds what your current plan allows.'}
				{#if contactEmail}
					<br />
					To discuss a plan that fits, reach us at
					<a href="mailto:{contactEmail}" class="text-primary">{contactEmail}</a>.
				{/if}
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel disabled={requesting}>Not now</AlertDialog.Cancel>
			<Button onclick={requestUpgrade} disabled={requesting}>
				{requesting ? 'Sending…' : 'Request an upgrade'}
			</Button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
