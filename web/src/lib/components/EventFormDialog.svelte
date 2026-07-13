<script lang="ts">
	import { ApiError, events as eventsApi } from '$lib/api';
	import type { CreateEvent, Event } from '$lib/api';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { toast } from 'svelte-sonner';

	let {
		open = $bindable(false),
		event = null,
		onsaved,
		onlimit
	}: {
		open?: boolean;
		event?: Event | null;
		onsaved?: (e: Event) => void;
		/** Called when the server rejects with a plan limit (HTTP 402). */
		onlimit?: (message: string) => void;
	} = $props();

	const blank: CreateEvent = {
		bride_name: '',
		bride_family_name: '',
		groom_name: '',
		groom_family_name: '',
		event_date: '',
		start_time: '10:00',
		end_time: '15:00',
		hall_name: '',
		venue_name: '',
		rsvp_by: ''
	};

	let form = $state<CreateEvent>({ ...blank });
	let saving = $state(false);

	// Seed the form each time the dialog opens.
	$effect(() => {
		if (open) form = event ? toForm(event) : { ...blank };
	});

	function toForm(e: Event): CreateEvent {
		const { id: _id, ...rest } = e;
		return { ...rest };
	}

	function validate(): string | null {
		const f = form;
		if (!f.bride_name.trim() || !f.groom_name.trim()) return 'Bride and groom names are required.';
		if (!f.event_date || !f.rsvp_by) return 'Event date and RSVP-by date are required.';
		if (f.end_time <= f.start_time) return 'End time must be after start time.';
		if (f.rsvp_by > f.event_date) return 'RSVP-by date must be on or before the event date.';
		return null;
	}

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		const err = validate();
		if (err) {
			toast.error(err);
			return;
		}
		saving = true;
		try {
			const saved = event ? await eventsApi.update(event.id, form) : await eventsApi.create(form);
			toast.success(event ? 'Event updated.' : 'Event created.');
			open = false;
			onsaved?.(saved);
		} catch (err2) {
			// A plan limit (402) is handed to the parent to show the upgrade dialog.
			if (err2 instanceof ApiError && err2.status === 402) {
				open = false;
				onlimit?.(err2.message);
			} else {
				toast.error(err2 instanceof ApiError ? err2.message : 'Could not save event');
			}
		} finally {
			saving = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-xl">
		<Dialog.Header>
			<Dialog.Title>{event ? 'Edit event' : 'New event'}</Dialog.Title>
		</Dialog.Header>
		<form onsubmit={submit} class="grid gap-4">
			<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
				<div class="grid gap-1.5">
					<Label for="bn">Bride's name</Label>
					<Input id="bn" bind:value={form.bride_name} />
				</div>
				<div class="grid gap-1.5">
					<Label for="bf">Bride's family name</Label>
					<Input id="bf" bind:value={form.bride_family_name} />
				</div>
				<div class="grid gap-1.5">
					<Label for="gn">Groom's name</Label>
					<Input id="gn" bind:value={form.groom_name} />
				</div>
				<div class="grid gap-1.5">
					<Label for="gf">Groom's family name</Label>
					<Input id="gf" bind:value={form.groom_family_name} />
				</div>
				<div class="grid gap-1.5">
					<Label for="ed">Event date</Label>
					<Input id="ed" type="date" bind:value={form.event_date} />
				</div>
				<div class="grid gap-1.5">
					<Label for="rb">RSVP by</Label>
					<Input id="rb" type="date" bind:value={form.rsvp_by} />
				</div>
				<div class="grid gap-1.5">
					<Label for="st">Start time</Label>
					<Input id="st" type="time" bind:value={form.start_time} />
				</div>
				<div class="grid gap-1.5">
					<Label for="et">End time</Label>
					<Input id="et" type="time" bind:value={form.end_time} />
				</div>
				<div class="grid gap-1.5">
					<Label for="hn">Hall name</Label>
					<Input id="hn" bind:value={form.hall_name} />
				</div>
				<div class="grid gap-1.5">
					<Label for="vn">Venue name</Label>
					<Input id="vn" bind:value={form.venue_name} />
				</div>
			</div>
			<Dialog.Footer>
				<Button type="button" variant="secondary" onclick={() => (open = false)}>Cancel</Button>
				<Button type="submit" disabled={saving}>
					{event ? 'Save changes' : 'Create event'}
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>
