<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		ApiError,
		config,
		events as eventsApi,
		guests as guestsApi,
		invites,
		saveBlob
	} from '$lib/api';
	import type { BatchSendReport, Event, Guest, InviteChannel, RsvpStatus } from '$lib/api';
	import { authStore } from '$lib/stores/auth.svelte';
	import {
		computeStats,
		deleteMany,
		filterSortGuests,
		moveMany,
		sendSelected,
		type GuestFilter,
		type SortKey
	} from '$lib/services';
	import { coupleTitle, fmtDate } from '$lib/format';
	import StatBar from '$lib/components/StatBar.svelte';
	import GuestTable from '$lib/components/GuestTable.svelte';
	import EventFormDialog from '$lib/components/EventFormDialog.svelte';
	import GuestFormDialog from '$lib/components/GuestFormDialog.svelte';
	import CsvImportDialog from '$lib/components/CsvImportDialog.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import LimitReachedDialog from '$lib/components/LimitReachedDialog.svelte';
	import SendReportDialog from '$lib/components/SendReportDialog.svelte';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Select from '$lib/components/ui/select';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { toast } from 'svelte-sonner';
	import {
		ArrowLeft,
		ChevronDown,
		Download,
		Mail,
		Pencil,
		Plus,
		Repeat,
		Search,
		Trash2,
		Upload,
		UserPlus,
		X
	} from '@lucide/svelte';

	const eventId = $derived(page.params.id ?? '');

	let event = $state<Event | null>(null);
	let guests = $state<Guest[]>([]);
	let loading = $state(true);

	let search = $state('');
	let filterStatus = $state<'all' | RsvpStatus>('all');
	let sortKey = $state<SortKey>('name');
	let sortDir = $state<'asc' | 'desc'>('asc');
	let selectedIds = $state<string[]>([]);

	let showEditEvent = $state(false);
	let showGuestForm = $state(false);
	let editingGuest = $state<Guest | null>(null);
	let showCsv = $state(false);
	let showReport = $state(false);
	let report = $state<BatchSendReport | null>(null);
	let quickAddName = $state('');
	let quickAddChannel = $state<InviteChannel>('einvite');

	let confirm = $state<{
		open: boolean;
		title: string;
		description: string;
		label: string;
		action: () => void | Promise<void>;
	}>({ open: false, title: '', description: '', label: 'Confirm', action: () => {} });

	function ask(title: string, description: string, label: string, action: () => void | Promise<void>) {
		confirm = { open: true, title, description, label, action };
	}

	let showLimit = $state(false);
	let limitMessage = $state('');
	let contactEmail = $state('');

	async function onLimit(message: string) {
		limitMessage = message;
		contactEmail = (await config.get().catch(() => ({ contact_email: '' }))).contact_email;
		showLimit = true;
	}

	async function loadAll() {
		loading = true;
		try {
			const [ev, gs] = await Promise.all([eventsApi.get(eventId), guestsApi.list(eventId)]);
			event = ev;
			guests = gs;
		} catch (err) {
			toast.error(err instanceof ApiError ? err.message : 'Could not load event');
		} finally {
			loading = false;
		}
	}
	async function refresh() {
		guests = await guestsApi.list(eventId);
	}
	onMount(loadAll);

	const stats = $derived(computeStats(guests));
	const filter = $derived<GuestFilter>({ search, channel: 'all', status: filterStatus });
	const view = $derived(filterSortGuests(guests, filter, sortKey, sortDir));
	const allSelected = $derived(view.length > 0 && view.every((g) => selectedIds.includes(g.id)));
	const hasSelection = $derived(selectedIds.length > 0);
	const selectedHasEinvite = $derived(
		guests.some((g) => selectedIds.includes(g.id) && g.channel === 'einvite')
	);
	const statusLabels: Record<string, string> = {
		all: 'All statuses',
		pending: 'Pending',
		attending: 'Attending',
		declined: 'Declined'
	};

	function toggle(id: string) {
		selectedIds = selectedIds.includes(id)
			? selectedIds.filter((x) => x !== id)
			: [...selectedIds, id];
	}
	function toggleAll() {
		const ids = view.map((g) => g.id);
		selectedIds = ids.every((id) => selectedIds.includes(id))
			? selectedIds.filter((id) => !ids.includes(id))
			: Array.from(new Set([...selectedIds, ...ids]));
	}
	function sort(key: SortKey) {
		if (sortKey === key) sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		else {
			sortKey = key;
			sortDir = 'asc';
		}
	}

	async function sendGuest(g: Guest) {
		try {
			await invites.sendGuest(eventId, g.id);
			toast.success(`Invite sent to ${g.name}.`);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Send failed');
		}
	}
	async function downloadGuest(g: Guest) {
		try {
			const { blob, filename } = await invites.guestPdf(eventId, g.id);
			saveBlob(blob, filename);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Download failed');
		}
	}
	async function moveGuest(g: Guest) {
		const target: InviteChannel = g.channel === 'einvite' ? 'print' : 'einvite';
		try {
			await guestsApi.update(eventId, g.id, { channel: target });
			await refresh();
			toast.success(`${g.name} moved to ${target === 'einvite' ? 'E-invite' : 'Print'}.`);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Move failed');
		}
	}
	function editGuest(g: Guest) {
		editingGuest = g;
		showGuestForm = true;
	}
	function deleteGuest(g: Guest) {
		ask('Delete guest?', `${g.name} will be permanently removed.`, 'Delete', async () => {
			await guestsApi.remove(eventId, g.id);
			await refresh();
			toast.success('Guest deleted.');
		});
	}
	function openAddGuest() {
		editingGuest = null;
		showGuestForm = true;
	}
	async function quickAdd() {
		const name = quickAddName.trim();
		if (!name) return;
		try {
			await guestsApi.create(eventId, { name, channel: quickAddChannel, max_party_size: 1 });
			quickAddName = '';
			await refresh();
		} catch (e) {
			if (e instanceof ApiError && e.status === 402) onLimit(e.message);
			else toast.error(e instanceof ApiError ? e.message : 'Could not add guest');
		}
	}

	function moveSelected(channel: InviteChannel) {
		const label = channel === 'einvite' ? 'E-invite' : 'Print';
		ask(
			`Move to ${label}?`,
			`${selectedIds.length} guest(s) will switch to the ${label} channel.`,
			'Move',
			async () => {
				const { ok } = await moveMany(eventId, selectedIds, channel);
				await refresh();
				selectedIds = [];
				toast.success(`Moved ${ok} guest(s).`);
			}
		);
	}
	function deleteSelected() {
		ask(
			'Delete selected guests?',
			`${selectedIds.length} guest(s) will be permanently removed.`,
			'Delete',
			async () => {
				const { ok } = await deleteMany(eventId, selectedIds);
				await refresh();
				selectedIds = [];
				toast.success(`Deleted ${ok} guest(s).`);
			}
		);
	}
	function sendSelectedGuests() {
		ask('Send e-invites?', 'Send to the selected e-invite guests.', 'Send', async () => {
			const sel = guests.filter((g) => selectedIds.includes(g.id));
			report = await sendSelected(eventId, sel);
			selectedIds = [];
			showReport = true;
		});
	}
	function sendAll() {
		ask('Send all e-invites?', `Send to all ${stats.einvite} e-invite guests.`, 'Send', async () => {
			try {
				report = await invites.sendBatch(eventId);
				showReport = true;
				await refresh();
			} catch (e) {
				toast.error(e instanceof ApiError ? e.message : 'Send failed');
			}
		});
	}
	async function downloadPrintAll() {
		try {
			const { blob, filename } = await invites.printBatch(eventId);
			saveBlob(blob, filename);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No print guests to download');
		}
	}
	function deleteEvent() {
		ask(
			'Delete event?',
			'This event and all its guests will be permanently removed.',
			'Delete',
			async () => {
				await eventsApi.remove(eventId);
				toast.success('Event deleted.');
				goto('/events');
			}
		);
	}
</script>

<main class="mx-auto w-full max-w-[1240px] px-6 py-7 pb-16">
	<a href="/events" class="text-muted-foreground mb-4 inline-flex items-center gap-1.5 text-[13px]">
		<ArrowLeft class="size-3.5" /> All events
	</a>

	{#if loading}
		<Skeleton class="h-8 w-72" />
		<Skeleton class="mt-6 h-16 w-full" />
		<Skeleton class="mt-6 h-64 w-full" />
	{:else if event}
		<div class="flex flex-wrap items-start justify-between gap-4">
			<div>
				<h1 class="text-[30px]">{coupleTitle(event)}</h1>
				<p class="text-muted-foreground mt-1 text-sm">
					{fmtDate(event.event_date)} · {event.venue_name} · RSVP by {fmtDate(event.rsvp_by)}
				</p>
			</div>
			<div class="flex gap-2">
				<Button variant="ghost" onclick={() => (showEditEvent = true)}>
					Edit event <Pencil class="size-4" />
				</Button>
				<Button variant="ghost" onclick={deleteEvent} aria-label="Delete event">
					<Trash2 class="size-4" />
				</Button>
			</div>
		</div>

		<div class="my-6"><StatBar {stats} /></div>
		<div class="border-t-2"></div>

		<!-- Toolbar -->
		<div class="my-4">
			{#if hasSelection}
				<div
					class="border-primary/40 bg-primary/5 flex flex-wrap items-center gap-3 border px-4 py-3"
				>
					<span class="text-[13px] font-semibold">{selectedIds.length} selected</span>
					<Button variant="secondary" onclick={() => moveSelected('print')}>Move to print</Button>
					<Button variant="secondary" onclick={() => moveSelected('einvite')}>Move to e-invite</Button>
					{#if selectedHasEinvite}
						<Button variant="secondary" onclick={sendSelectedGuests}>Send e-invite</Button>
					{/if}
					<Button variant="ghost" onclick={deleteSelected}>Delete</Button>
					<span class="flex-1"></span>
					<Button variant="ghost" onclick={() => (selectedIds = [])}>
						Clear <X class="size-4" />
					</Button>
				</div>
			{:else}
				<div class="flex flex-wrap items-center gap-3">
					<div class="relative w-60">
						<Search class="text-muted-foreground absolute top-2.5 left-2.5 size-3.5" />
						<Input class="pl-8" placeholder="Search guests" bind:value={search} />
					</div>
					<Select.Root type="single" bind:value={filterStatus}>
						<Select.Trigger class="w-[150px]">{statusLabels[filterStatus]}</Select.Trigger>
						<Select.Content>
							<Select.Item value="all">All statuses</Select.Item>
							<Select.Item value="pending">Pending</Select.Item>
							<Select.Item value="attending">Attending</Select.Item>
							<Select.Item value="declined">Declined</Select.Item>
						</Select.Content>
					</Select.Root>
					<span class="flex-1"></span>
					<DropdownMenu.Root>
						<DropdownMenu.Trigger class={buttonVariants({ variant: 'secondary' })}>
							Add guest <ChevronDown class="size-4" />
						</DropdownMenu.Trigger>
						<DropdownMenu.Content align="end">
							<DropdownMenu.Item onclick={openAddGuest}>
								<UserPlus class="size-4" /> Single guest
							</DropdownMenu.Item>
							<DropdownMenu.Item onclick={() => (showCsv = true)}>
								<Upload class="size-4" /> Import list (CSV)
							</DropdownMenu.Item>
						</DropdownMenu.Content>
					</DropdownMenu.Root>
					<DropdownMenu.Root>
						<DropdownMenu.Trigger class={buttonVariants({ variant: 'default' })}>
							Bulk send <ChevronDown class="size-4" />
						</DropdownMenu.Trigger>
						<DropdownMenu.Content align="end">
							<DropdownMenu.Item onclick={sendAll}>
								<Mail class="size-4" /> Send all e-invites ({stats.einvite})
							</DropdownMenu.Item>
							<DropdownMenu.Item onclick={downloadPrintAll}>
								<Download class="size-4" /> Download print PDFs ({stats.print})
							</DropdownMenu.Item>
						</DropdownMenu.Content>
					</DropdownMenu.Root>
				</div>
			{/if}
		</div>

		<GuestTable
			guests={view}
			{selectedIds}
			{allSelected}
			{sortKey}
			{sortDir}
			onToggle={toggle}
			onToggleAll={toggleAll}
			onSort={sort}
			onSend={sendGuest}
			onDownload={downloadGuest}
			onMove={moveGuest}
			onEdit={editGuest}
			onDelete={deleteGuest}
		/>

		<!-- Quick add -->
		<div class="mt-4 flex flex-wrap items-center gap-3 border-t-2 pt-4">
			<Input class="max-w-[280px]" placeholder="Quick add: guest name" bind:value={quickAddName} />
			<div class="inline-flex border">
				<button
					type="button"
					class="px-3 py-1.5 text-[13px] {quickAddChannel === 'einvite'
						? 'bg-primary text-primary-foreground'
						: 'hover:bg-accent/50'}"
					onclick={() => (quickAddChannel = 'einvite')}>E-invite</button
				>
				<button
					type="button"
					class="border-l px-3 py-1.5 text-[13px] {quickAddChannel === 'print'
						? 'bg-primary text-primary-foreground'
						: 'hover:bg-accent/50'}"
					onclick={() => (quickAddChannel = 'print')}>Print</button
				>
			</div>
			<Button onclick={quickAdd} disabled={!quickAddName.trim()}>
				Add guest <Plus class="size-4" />
			</Button>
		</div>
	{/if}
</main>

{#if event}
	<EventFormDialog bind:open={showEditEvent} {event} onsaved={(e) => (event = e)} />
{/if}
<GuestFormDialog
	bind:open={showGuestForm}
	{eventId}
	guest={editingGuest}
	onsaved={refresh}
	onlimit={onLimit}
/>

<LimitReachedDialog
	bind:open={showLimit}
	plan={authStore.user?.plan ?? 'free'}
	{contactEmail}
	message={limitMessage}
/>
<CsvImportDialog bind:open={showCsv} {eventId} onimported={refresh} />
<ConfirmDialog
	bind:open={confirm.open}
	title={confirm.title}
	description={confirm.description}
	confirmLabel={confirm.label}
	onconfirm={confirm.action}
/>
<SendReportDialog bind:open={showReport} {report} />
