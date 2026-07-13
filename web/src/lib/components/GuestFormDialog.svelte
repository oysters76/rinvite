<script lang="ts">
	import { ApiError, guests as guestsApi } from '$lib/api';
	import type { Guest, InviteChannel } from '$lib/api';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { toast } from 'svelte-sonner';

	let {
		open = $bindable(false),
		eventId,
		guest = null,
		onsaved,
		onlimit
	}: {
		open?: boolean;
		eventId: string;
		guest?: Guest | null;
		onsaved?: () => void;
		/** Called when the server rejects with a plan limit (HTTP 402). */
		onlimit?: (message: string) => void;
	} = $props();

	let name = $state('');
	let channel = $state<InviteChannel>('einvite');
	let email = $state('');
	let phone = $state('');
	let maxParty = $state(1);
	let saving = $state(false);

	$effect(() => {
		if (open) {
			name = guest?.name ?? '';
			channel = guest?.channel ?? 'einvite';
			email = guest?.email ?? '';
			phone = guest?.phone ?? '';
			maxParty = guest?.max_party_size ?? 1;
		}
	});

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		if (!name.trim()) {
			toast.error('Guest name is required.');
			return;
		}
		const payload = {
			name: name.trim(),
			channel,
			email: email.trim() || null,
			phone: phone.trim() || null,
			max_party_size: Math.max(1, Number(maxParty) || 1)
		};
		saving = true;
		try {
			if (guest) await guestsApi.update(eventId, guest.id, payload);
			else await guestsApi.create(eventId, payload);
			toast.success(guest ? 'Guest updated.' : 'Guest added.');
			open = false;
			onsaved?.();
		} catch (err) {
			// A plan limit (402) is handed to the parent to show the upgrade dialog.
			if (err instanceof ApiError && err.status === 402) {
				open = false;
				onlimit?.(err.message);
			} else {
				toast.error(err instanceof ApiError ? err.message : 'Could not save guest');
			}
		} finally {
			saving = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>{guest ? 'Edit guest' : 'Add guest'}</Dialog.Title>
		</Dialog.Header>
		<form onsubmit={submit} class="grid gap-4">
			<div class="grid gap-1.5">
				<Label for="g-name">Guest name</Label>
				<Input id="g-name" bind:value={name} />
			</div>
			<div class="grid gap-1.5">
				<Label>Channel</Label>
				<div class="inline-flex w-full border">
					<button
						type="button"
						class="flex-1 py-2 text-[13px] {channel === 'einvite'
							? 'bg-primary text-primary-foreground'
							: 'hover:bg-accent/50'}"
						onclick={() => (channel = 'einvite')}
					>
						E-invite
					</button>
					<button
						type="button"
						class="flex-1 border-l py-2 text-[13px] {channel === 'print'
							? 'bg-primary text-primary-foreground'
							: 'hover:bg-accent/50'}"
						onclick={() => (channel = 'print')}
					>
						Print
					</button>
				</div>
			</div>
			<div class="grid gap-1.5">
				<Label for="g-email">Email</Label>
				<Input id="g-email" type="email" bind:value={email} />
			</div>
			<div class="grid gap-1.5">
				<Label for="g-phone">Phone</Label>
				<Input id="g-phone" bind:value={phone} />
			</div>
			<div class="grid gap-1.5">
				<Label for="g-max">Max party size</Label>
				<Input id="g-max" type="number" min="1" bind:value={maxParty} />
			</div>
			<Dialog.Footer>
				<Button type="button" variant="secondary" onclick={() => (open = false)}>Cancel</Button>
				<Button type="submit" disabled={saving}>{guest ? 'Save changes' : 'Add guest'}</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
