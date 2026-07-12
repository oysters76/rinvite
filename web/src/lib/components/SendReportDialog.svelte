<script lang="ts">
	import type { BatchSendReport } from '$lib/api';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import Tag from './Tag.svelte';

	let {
		open = $bindable(false),
		report
	}: { open?: boolean; report: BatchSendReport | null } = $props();
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>Send report</Dialog.Title>
		</Dialog.Header>
		{#if report}
			<p class="text-sm">
				{report.total} attempted · {report.sent} sent · {report.failed} failed.
			</p>
			<div class="mt-1 max-h-[260px] divide-y overflow-auto border-t-2">
				{#each report.results as r (r.guest_id)}
					<div class="flex items-center justify-between py-2">
						<span class="text-[13px]">{r.guest_name}</span>
						<Tag variant={r.status === 'sent' ? 'accent' : 'outline'}>
							{r.status === 'sent' ? 'Sent' : 'Failed'}
						</Tag>
					</div>
				{/each}
			</div>
		{/if}
		<Dialog.Footer>
			<Button onclick={() => (open = false)}>Done</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
