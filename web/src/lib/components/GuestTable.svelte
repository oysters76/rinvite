<script lang="ts">
	import type { Guest } from '$lib/api';
	import type { SortDir, SortKey } from '$lib/services';
	import * as Table from '$lib/components/ui/table';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import ChannelBadge from './ChannelBadge.svelte';
	import RsvpBadge from './RsvpBadge.svelte';
	import { ArrowDown, ArrowUp, Download, Mail, Pencil, Repeat, Trash2 } from '@lucide/svelte';

	let {
		guests,
		selectedIds,
		allSelected,
		sortKey,
		sortDir,
		onToggle,
		onToggleAll,
		onSort,
		onSend,
		onDownload,
		onMove,
		onEdit,
		onDelete
	}: {
		guests: Guest[];
		selectedIds: string[];
		allSelected: boolean;
		sortKey: SortKey;
		sortDir: SortDir;
		onToggle: (id: string) => void;
		onToggleAll: () => void;
		onSort: (key: SortKey) => void;
		onSend: (g: Guest) => void;
		onDownload: (g: Guest) => void;
		onMove: (g: Guest) => void;
		onEdit: (g: Guest) => void;
		onDelete: (g: Guest) => void;
	} = $props();

	const isSelected = (id: string) => selectedIds.includes(id);
</script>

{#snippet sortArrow(key: SortKey)}
	{#if sortKey === key}
		{#if sortDir === 'asc'}<ArrowUp class="size-3" />{:else}<ArrowDown class="size-3" />{/if}
	{/if}
{/snippet}

<Table.Root>
	<Table.Header>
		<Table.Row>
			<Table.Head class="w-9">
				<Checkbox checked={allSelected} onCheckedChange={onToggleAll} aria-label="Select all" />
			</Table.Head>
			<Table.Head>
				<button class="inline-flex items-center gap-1 hover:text-primary" onclick={() => onSort('name')}>
					Name {@render sortArrow('name')}
				</button>
			</Table.Head>
			<Table.Head>Contact</Table.Head>
			<Table.Head>Channel</Table.Head>
			<Table.Head>
				<button
					class="inline-flex items-center gap-1 hover:text-primary"
					onclick={() => onSort('max_party_size')}
				>
					Party {@render sortArrow('max_party_size')}
				</button>
			</Table.Head>
			<Table.Head>
				<button
					class="inline-flex items-center gap-1 hover:text-primary"
					onclick={() => onSort('rsvp_status')}
				>
					RSVP {@render sortArrow('rsvp_status')}
				</button>
			</Table.Head>
			<Table.Head class="w-[120px]">Actions</Table.Head>
		</Table.Row>
	</Table.Header>
	<Table.Body>
		{#each guests as g (g.id)}
			<Table.Row>
				<Table.Cell>
					<Checkbox
						checked={isSelected(g.id)}
						onCheckedChange={() => onToggle(g.id)}
						aria-label="Select {g.name}"
					/>
				</Table.Cell>
				<Table.Cell class="font-medium">{g.name}</Table.Cell>
				<Table.Cell class="text-muted-foreground">{g.email || g.phone || '—'}</Table.Cell>
				<Table.Cell><ChannelBadge channel={g.channel} /></Table.Cell>
				<Table.Cell>{g.party_size ?? '—'} / {g.max_party_size}</Table.Cell>
				<Table.Cell><RsvpBadge status={g.rsvp_status} /></Table.Cell>
				<Table.Cell>
					<div class="flex gap-0.5">
						{#if g.channel === 'einvite'}
							<button class="hover:bg-accent p-1.5" aria-label="Send" onclick={() => onSend(g)}>
								<Mail class="size-4" />
							</button>
						{:else}
							<button class="hover:bg-accent p-1.5" aria-label="Download PDF" onclick={() => onDownload(g)}>
								<Download class="size-4" />
							</button>
						{/if}
						<button class="hover:bg-accent p-1.5" aria-label="Switch channel" onclick={() => onMove(g)}>
							<Repeat class="size-4" />
						</button>
						<button class="hover:bg-accent p-1.5" aria-label="Edit" onclick={() => onEdit(g)}>
							<Pencil class="size-4" />
						</button>
						<button class="hover:bg-accent p-1.5" aria-label="Delete" onclick={() => onDelete(g)}>
							<Trash2 class="size-4" />
						</button>
					</div>
				</Table.Cell>
			</Table.Row>
		{/each}
	</Table.Body>
</Table.Root>

{#if guests.length === 0}
	<p class="text-muted-foreground py-4 text-[13px]">No guests match these filters.</p>
{/if}
