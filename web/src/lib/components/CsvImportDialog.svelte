<script lang="ts">
	import { ApiError, guests as guestsApi, saveBlob } from '$lib/api';
	import type { CreateGuest } from '$lib/api';
	import { parseGuestCsvFile, validateGuestRow } from '$lib/services';
	import * as Dialog from '$lib/components/ui/dialog';
	import * as Table from '$lib/components/ui/table';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { toast } from 'svelte-sonner';
	import { Plus, Trash2, Upload } from '@lucide/svelte';

	let {
		open = $bindable(false),
		eventId,
		onimported,
		onlimit
	}: {
		open?: boolean;
		eventId: string;
		onimported?: () => void;
		/** Called when the server rejects the batch with a plan limit (HTTP 402). */
		onlimit?: (message: string) => void;
	} = $props();

	let rows = $state<CreateGuest[]>([]);
	let fileName = $state('');
	let importing = $state(false);
	let fileInput: HTMLInputElement | null = $state(null);

	// Reset whenever the dialog is (re)opened.
	$effect(() => {
		if (open) {
			rows = [];
			fileName = '';
			importing = false;
			if (fileInput) fileInput.value = '';
		}
	});

	const rowErrors = $derived(rows.map(validateGuestRow));
	const invalidCount = $derived(rowErrors.filter((e) => e.length > 0).length);
	const canImport = $derived(rows.length > 0 && invalidCount === 0 && !importing);

	function onFile(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		fileName = file.name;
		const reader = new FileReader();
		reader.onload = () => {
			const parsed = parseGuestCsvFile(String(reader.result ?? ''));
			if (!parsed.length) {
				toast.error('No guests found. The file needs a header row with a "name" column.');
				rows = [];
				return;
			}
			rows = parsed;
		};
		reader.onerror = () => toast.error('Could not read that file.');
		reader.readAsText(file);
	}

	function blankRow(): CreateGuest {
		return { name: '', channel: 'einvite', email: null, phone: null, max_party_size: 1 };
	}

	function addRow() {
		rows = [...rows, blankRow()];
	}

	function removeRow(i: number) {
		rows = rows.filter((_, idx) => idx !== i);
	}

	function downloadTemplate() {
		const csv = 'name,channel,email,phone,max_party_size\n';
		saveBlob(new Blob([csv], { type: 'text/csv' }), 'guest-list-template.csv');
	}

	async function submit() {
		if (!canImport) return;
		importing = true;
		try {
			const created = await guestsApi.bulkCreate(eventId, rows);
			toast.success(`${created.length} guest${created.length === 1 ? '' : 's'} imported.`);
			open = false;
			onimported?.();
		} catch (err) {
			// A plan limit (402) is handed to the parent to show the upgrade dialog.
			if (err instanceof ApiError && err.status === 402) {
				open = false;
				onlimit?.(err.message);
			} else {
				toast.error(err instanceof ApiError ? err.message : 'Could not import guests');
			}
		} finally {
			importing = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-3xl">
		<Dialog.Header>
			<Dialog.Title>Import guest list</Dialog.Title>
		</Dialog.Header>

		<div class="flex flex-wrap items-center gap-3">
			<Button type="button" variant="secondary" onclick={() => fileInput?.click()}>
				<Upload class="mr-2 size-4" />
				Choose CSV file
			</Button>
			<input
				bind:this={fileInput}
				type="file"
				accept=".csv,text/csv"
				class="hidden"
				onchange={onFile}
			/>
			{#if fileName}
				<span class="text-muted-foreground text-xs">{fileName}</span>
			{/if}
			<button
				type="button"
				class="text-muted-foreground hover:text-foreground ml-auto text-xs underline"
				onclick={downloadTemplate}
			>
				Download template
			</button>
		</div>
		<p class="text-muted-foreground text-xs">
			The file needs a header row. Columns: <code>name</code> (required),
			<code>channel</code> (<code>einvite</code> or <code>print</code>), <code>email</code>,
			<code>phone</code>, <code>max_party_size</code>. Review and edit below before importing.
		</p>

		{#if rows.length}
			<div class="max-h-[50vh] overflow-auto border">
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head class="w-[28%]">Name</Table.Head>
							<Table.Head class="w-[15%]">Channel</Table.Head>
							<Table.Head>Email</Table.Head>
							<Table.Head>Phone</Table.Head>
							<Table.Head class="w-[10%]">Party</Table.Head>
							<Table.Head class="w-10"></Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each rows as row, i (i)}
							{@const errs = rowErrors[i]}
							<Table.Row class={errs.length ? 'bg-destructive/5' : ''}>
								<Table.Cell>
									<Input
										bind:value={row.name}
										aria-invalid={errs.some((e) => e.includes('name'))}
										class="h-8"
									/>
								</Table.Cell>
								<Table.Cell>
									<select
										bind:value={row.channel}
										class="border-input bg-background h-8 w-full border px-2 text-sm"
									>
										<option value="einvite">E-invite</option>
										<option value="print">Print</option>
									</select>
								</Table.Cell>
								<Table.Cell>
									<Input
										bind:value={row.email}
										aria-invalid={errs.some((e) => e.includes('email'))}
										class="h-8"
									/>
								</Table.Cell>
								<Table.Cell>
									<Input
										bind:value={row.phone}
										aria-invalid={errs.some((e) => e.includes('phone'))}
										class="h-8"
									/>
								</Table.Cell>
								<Table.Cell>
									<Input
										type="number"
										min="1"
										bind:value={row.max_party_size}
										aria-invalid={errs.some((e) => e.includes('party'))}
										class="h-8"
									/>
								</Table.Cell>
								<Table.Cell>
									<button
										type="button"
										class="text-muted-foreground hover:text-destructive"
										title="Remove row"
										onclick={() => removeRow(i)}
									>
										<Trash2 class="size-4" />
									</button>
								</Table.Cell>
							</Table.Row>
							{#if errs.length}
								<Table.Row class="bg-destructive/5">
									<Table.Cell colspan={6} class="text-destructive pt-0 text-xs">
										Row {i + 1}: {errs.join('; ')}
									</Table.Cell>
								</Table.Row>
							{/if}
						{/each}
					</Table.Body>
				</Table.Root>
			</div>

			<div class="flex items-center justify-between">
				<Button type="button" variant="ghost" size="sm" onclick={addRow}>
					<Plus class="mr-1 size-4" /> Add row
				</Button>
				<span class="text-muted-foreground text-xs">
					{rows.length} row{rows.length === 1 ? '' : 's'}{invalidCount
						? ` · ${invalidCount} need${invalidCount === 1 ? 's' : ''} fixing`
						: ''}
				</span>
			</div>
		{/if}

		<Dialog.Footer>
			<Button type="button" variant="secondary" onclick={() => (open = false)}>Cancel</Button>
			<Button onclick={submit} disabled={!canImport}>
				Import{rows.length ? ` ${rows.length}` : ''} guest{rows.length === 1 ? '' : 's'}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
