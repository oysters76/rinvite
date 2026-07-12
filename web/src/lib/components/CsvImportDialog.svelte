<script lang="ts">
	import { parseGuestCsv, importMany } from '$lib/services';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Textarea } from '$lib/components/ui/textarea';
	import { toast } from 'svelte-sonner';

	let {
		open = $bindable(false),
		eventId,
		onimported
	}: { open?: boolean; eventId: string; onimported?: () => void } = $props();

	let text = $state('');
	let importing = $state(false);

	const rows = $derived(parseGuestCsv(text));
	const preview = $derived(
		rows.length ? `${rows.length} guest${rows.length === 1 ? '' : 's'} ready to import.` : 'Paste guests above to preview.'
	);

	$effect(() => {
		if (open) text = '';
	});

	async function submit() {
		if (!rows.length) return;
		importing = true;
		try {
			const { ok, failed } = await importMany(eventId, rows);
			toast.success(`${ok} guest${ok === 1 ? '' : 's'} imported${failed ? `, ${failed} failed` : ''}.`);
			open = false;
			onimported?.();
		} finally {
			importing = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-xl">
		<Dialog.Header>
			<Dialog.Title>Import guest list</Dialog.Title>
		</Dialog.Header>
		<p class="text-muted-foreground text-xs">
			One guest per line: <code>name, channel, email, phone, max party size</code>. Channel is
			<code>einvite</code> or <code>print</code>.
		</p>
		<Textarea
			rows={8}
			bind:value={text}
			placeholder={'Mr Dhammika & family, einvite, d@example.com, , 2\nAunty Kamala, print, , 0771234567, 1'}
		/>
		<p class="text-muted-foreground text-xs">{preview}</p>
		<Dialog.Footer>
			<Button type="button" variant="secondary" onclick={() => (open = false)}>Cancel</Button>
			<Button onclick={submit} disabled={rows.length === 0 || importing}>
				Import {rows.length} guests
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
