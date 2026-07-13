<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { loadEventsOverview, type EventOverview } from '$lib/services';
	import { ApiError, config } from '$lib/api';
	import EventCard from '$lib/components/EventCard.svelte';
	import EventFormDialog from '$lib/components/EventFormDialog.svelte';
	import LimitReachedDialog from '$lib/components/LimitReachedDialog.svelte';
	import { authStore } from '$lib/stores/auth.svelte';
	import { Button } from '$lib/components/ui/button';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { toast } from 'svelte-sonner';
	import { Plus } from '@lucide/svelte';

	let events = $state<EventOverview[] | null>(null);
	let showCreate = $state(false);

	let showLimit = $state(false);
	let limitMessage = $state('');
	let contactEmail = $state('');

	async function onLimit(message: string) {
		limitMessage = message;
		contactEmail = (await config.get().catch(() => ({ contact_email: '' }))).contact_email;
		showLimit = true;
	}

	async function refresh() {
		try {
			events = await loadEventsOverview();
		} catch (err) {
			toast.error(err instanceof ApiError ? err.message : 'Could not load events');
			events = [];
		}
	}

	onMount(refresh);

	const countLabel = $derived(
		events ? `${events.length} event${events.length === 1 ? '' : 's'}` : ''
	);
</script>

<main class="mx-auto w-full max-w-[1160px] px-6 py-8">
	<div class="flex items-start justify-between gap-4">
		<div>
			<h1 class="text-[32px]">Events</h1>
			<p class="text-muted-foreground mt-1 text-sm">{countLabel}</p>
		</div>
		<Button onclick={() => (showCreate = true)}>
			New event
			<Plus class="size-4" />
		</Button>
	</div>

	{#if events === null}
		<div class="mt-6 grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-5">
			{#each Array(3) as _, i (i)}
				<Skeleton class="h-48 w-full" />
			{/each}
		</div>
	{:else if events.length === 0}
		<p class="text-muted-foreground mt-10 text-sm">
			No events yet. Create your first one to start inviting guests.
		</p>
	{:else}
		<div class="mt-6 grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-5">
			{#each events as ev (ev.id)}
				<EventCard event={ev} onopen={() => goto(`/events/${ev.id}`)} />
			{/each}
		</div>
	{/if}
</main>

<EventFormDialog
	bind:open={showCreate}
	onsaved={(e) => goto(`/events/${e.id}`)}
	onlimit={onLimit}
/>

<LimitReachedDialog
	bind:open={showLimit}
	plan={authStore.user?.plan ?? 'free'}
	{contactEmail}
	message={limitMessage}
/>
