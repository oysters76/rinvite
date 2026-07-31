<script lang="ts">
	import { saveBlob } from '$lib/api';
	import type { Event as WeddingEvent } from '$lib/api';
	import { fmtDate } from '$lib/format';
	import {
		createCardEditor,
		ensureCardFonts,
		fileStem,
		coupleLine,
		DEFAULT_MESSAGE,
		MAX_GUEST_LEN,
		MAX_MESSAGE_LEN,
		CARD_W,
		CARD_H,
		type CardEditor,
		type CardModel
	} from '$lib/services/thankyou-card';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Textarea } from '$lib/components/ui/textarea';
	import { Label } from '$lib/components/ui/label';
	import { toast } from 'svelte-sonner';
	import { ImagePlus, RotateCcw, Upload } from '@lucide/svelte';

	let {
		open = $bindable(false),
		event
	}: {
		open?: boolean;
		/** Seeds the card: names, precedence and date are prefilled from here. */
		event?: WeddingEvent | null;
	} = $props();

	/** Reject anything that would sit in memory as a huge base64 data URL. */
	const MAX_FILE_BYTES = 15 * 1024 * 1024;
	const MAX_IMAGE_SIDE = 8000;

	// Only the guest name is expected to be typed; the rest is seeded from `event`.
	let guest = $state('');
	let bride = $state('');
	let groom = $state('');
	let precedence = $state<WeddingEvent['precedence']>('bride');
	let message = $state(DEFAULT_MESSAGE);
	let dateIso = $state('');
	let zoom = $state(1);
	let photoLoaded = $state(false);
	let generating = $state(false);
	let loadingEditor = $state(false);
	let hasDragged = $state(false);

	// Width of the preview column, so the 400x600 stage can be scaled to fit.
	let fitWidth = $state(0);
	const previewScale = $derived(fitWidth ? Math.min(1, (fitWidth - 16) / CARD_W) : 1);

	let containerEl = $state<HTMLDivElement | null>(null);
	let fileInput = $state<HTMLInputElement | null>(null);
	let editor: CardEditor | null = null;
	// Bumped on every open/close so a build that resolves after the dialog closed
	// can tell that it is stale, destroy itself and leave `editor` null.
	let generation = 0;

	const model = (): CardModel => ({
		bride,
		groom,
		precedence,
		guest,
		message,
		date: dateIso ? fmtDate(dateIso) : ''
	});

	/** Seed everything the app already knows, so only the guest name is typed. */
	function seedFromEvent() {
		bride = event?.bride_name ?? '';
		groom = event?.groom_name ?? '';
		precedence = event?.precedence ?? 'bride';
		dateIso = event?.event_date ?? '';
		message = DEFAULT_MESSAGE;
		guest = '';
		zoom = 1;
		photoLoaded = false;
		hasDragged = false;
		if (fileInput) fileInput.value = '';
	}

	function teardown() {
		editor?.destroy();
		editor = null;
	}

	// Build the Konva editor once the dialog is open and its container is mounted;
	// tear it down when the dialog closes or the component unmounts.
	$effect(() => {
		if (!open) {
			generation++;
			teardown();
			return;
		}
		if (!containerEl || editor || loadingEditor) return;

		const mine = ++generation;
		loadingEditor = true;
		seedFromEvent();
		createCardEditor(containerEl)
			.then((e) => {
				// Closed (or reopened) while we were loading — this stage is orphaned.
				if (mine !== generation) {
					e.destroy();
					return;
				}
				editor = e;
				editor.update(model());
			})
			.catch(() => toast.error('Could not open the card editor.'))
			.finally(() => {
				if (mine === generation) loadingEditor = false;
			});
	});

	// Fonts are ~2 MB; start them the moment the dialog opens rather than waiting
	// for the Konva chunk to land.
	$effect(() => {
		if (open) void ensureCardFonts().catch(() => {});
	});

	$effect(() => teardown);

	// Push text changes into the live preview. Build the model *before* the
	// optional call: `editor?.update(model())` short-circuits while the editor is
	// still loading, so model() never runs, nothing is tracked, and the effect
	// never fires again.
	$effect(() => {
		const next = model();
		editor?.update(next);
	});

	async function onFile(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		// Clear immediately so picking the same file again still fires `change`.
		input.value = '';
		if (!file) return;
		if (file.size > MAX_FILE_BYTES) {
			toast.error('That photo is too large. Please use one under 15 MB.');
			return;
		}
		try {
			const bitmapUrl = URL.createObjectURL(file);
			try {
				const img = await loadImage(bitmapUrl);
				if (img.naturalWidth > MAX_IMAGE_SIDE || img.naturalHeight > MAX_IMAGE_SIDE) {
					toast.error('That photo is too big to work with. Please resize it first.');
					return;
				}
				if (!editor) return;
				editor.setPhoto(img);
				zoom = 1;
				hasDragged = false;
				photoLoaded = editor.hasPhoto();
				if (!photoLoaded) toast.error('That image could not be loaded.');
			} finally {
				URL.revokeObjectURL(bitmapUrl);
			}
		} catch {
			toast.error('That image could not be loaded.');
		}
	}

	function loadImage(src: string): Promise<HTMLImageElement> {
		return new Promise((resolve, reject) => {
			const img = new Image();
			img.onload = () => resolve(img);
			img.onerror = () => reject(new Error('image load failed'));
			img.src = src;
		});
	}

	function onZoom() {
		editor?.setZoom(zoom);
	}

	function resetFraming() {
		zoom = 1;
		hasDragged = true;
		editor?.resetFraming();
	}

	/** Arrow keys nudge the framing, for anyone not using a pointer. */
	function onPreviewKey(e: KeyboardEvent) {
		if (!photoLoaded) return;
		const step = e.shiftKey ? 0.1 : 0.02;
		const moves: Record<string, [number, number]> = {
			ArrowLeft: [-step, 0],
			ArrowRight: [step, 0],
			ArrowUp: [0, -step],
			ArrowDown: [0, step]
		};
		const move = moves[e.key];
		if (!move) return;
		e.preventDefault();
		hasDragged = true;
		editor?.nudge(move[0], move[1]);
	}

	async function generate() {
		if (!editor || !photoLoaded) return;
		generating = true;
		try {
			const blob = await editor.toBlob();
			// Prefer the guest; fall back to the couple. An empty fallback lets the
			// `||` actually reach the second option.
			const stem = fileStem(guest, '') || fileStem(coupleLine(model()));
			saveBlob(blob, `thank-you-${stem}.png`);
			toast.success('Thank-you card downloaded.');
		} catch {
			toast.error('Could not generate the card.');
		} finally {
			generating = false;
		}
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-4xl">
		<Dialog.Header>
			<Dialog.Title>Thank-you card</Dialog.Title>
			<Dialog.Description>
				Upload a photo, drag it to frame it, then download a print-ready 4×6 card.
			</Dialog.Description>
		</Dialog.Header>

		<div class="grid gap-6 md:grid-cols-[1fr_minmax(0,400px)]">
			<!-- Controls -->
			<div class="order-2 flex flex-col gap-4 md:order-1">
				<div class="flex flex-col gap-1.5">
					<Label for="ty-guest">Guest name</Label>
					<Input
						id="ty-guest"
						bind:value={guest}
						maxlength={MAX_GUEST_LEN}
						autofocus
						placeholder="e.g. The Fernando Family"
					/>
					<p class="text-muted-foreground text-xs">
						Everything else is filled in from the event — edit it below if you need to.
					</p>
				</div>

				<div class="flex flex-wrap items-center gap-3">
					<Button type="button" variant="secondary" onclick={() => fileInput?.click()}>
						<Upload class="mr-2 size-4" />
						{photoLoaded ? 'Change photo' : 'Upload photo'}
					</Button>
					{#if photoLoaded}
						<Button type="button" variant="ghost" size="sm" onclick={resetFraming}>
							<RotateCcw class="mr-2 size-4" />
							Reset framing
						</Button>
					{/if}
					<input
						bind:this={fileInput}
						type="file"
						accept="image/*"
						class="hidden"
						onchange={onFile}
					/>
				</div>

				{#if photoLoaded}
					<div class="flex flex-col gap-1.5">
						<Label for="ty-zoom">Zoom</Label>
						<input
							id="ty-zoom"
							type="range"
							min="1"
							max="4"
							step="0.01"
							bind:value={zoom}
							oninput={onZoom}
							class="w-full"
						/>
						<p class="text-muted-foreground text-xs">
							Drag the photo in the preview to reposition it, or use the arrow keys.
						</p>
					</div>
				{/if}

				<details class="border-border rounded-md border p-3">
					<summary class="cursor-pointer text-sm font-medium">Card details</summary>
					<div class="mt-3 flex flex-col gap-4">
						<div class="grid gap-4 sm:grid-cols-2">
							<div class="flex flex-col gap-1.5">
								<Label for="ty-bride">Bride name</Label>
								<Input id="ty-bride" bind:value={bride} placeholder="e.g. Amaya" />
							</div>
							<div class="flex flex-col gap-1.5">
								<Label for="ty-groom">Groom name</Label>
								<Input id="ty-groom" bind:value={groom} placeholder="e.g. Nuwan" />
							</div>
						</div>

						<div class="flex flex-col gap-1.5">
							<Label for="ty-message">Message</Label>
							<Textarea
								id="ty-message"
								bind:value={message}
								rows={3}
								maxlength={MAX_MESSAGE_LEN}
								placeholder={DEFAULT_MESSAGE}
							/>
							<p class="text-muted-foreground text-xs">
								{message.length}/{MAX_MESSAGE_LEN}
							</p>
						</div>

						<div class="flex flex-col gap-1.5">
							<Label for="ty-date">Date</Label>
							<Input id="ty-date" type="date" bind:value={dateIso} />
						</div>
					</div>
				</details>
			</div>

			<!-- Live preview. Scales down on narrow screens rather than forcing a
			     400px horizontal scroller; Konva maps pointer coords through the
			     container's bounding rect, so dragging still lines up when scaled. -->
			<div class="order-1 md:order-2">
				<div
					bind:clientWidth={fitWidth}
					class="bg-muted/30 mx-auto border p-2"
					style="max-width:{CARD_W + 16}px"
				>
					<div
						class="relative"
						style="width:100%;height:{CARD_H * previewScale}px;overflow:hidden;"
					>
						<!--
							`application` is the right role for a directly-manipulable canvas:
							it tells assistive tech to pass keys through to us. Svelte classes
							it as non-interactive, but the keydown handler here IS the
							accessible route to framing the photo — dropping it to satisfy the
							rule would make this less usable, not more.
						-->
						<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
						<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
						<div
							bind:this={containerEl}
							role="application"
							aria-label="Card preview — drag or use the arrow keys to position the photo"
							tabindex="0"
							onkeydown={onPreviewKey}
							onpointerdown={() => (hasDragged = true)}
							style="width:{CARD_W}px;height:{CARD_H}px;transform:scale({previewScale});transform-origin:top left;"
						></div>
						{#if loadingEditor}
							<div
								class="bg-muted text-muted-foreground absolute inset-0 grid animate-pulse place-items-center text-sm"
							>
								Preparing the card…
							</div>
						{/if}
						{#if photoLoaded && !hasDragged}
							<div
								class="pointer-events-none absolute left-1/2 -translate-x-1/2 rounded-full bg-black/60 px-3 py-1 text-xs text-white"
								style="top:{205 * previewScale}px"
							>
								Drag to reposition
							</div>
						{/if}
					</div>
				</div>
			</div>
		</div>

		<Dialog.Footer>
			<Button type="button" variant="secondary" onclick={() => (open = false)}>Cancel</Button>
			<Button onclick={generate} disabled={!photoLoaded || generating}>
				<ImagePlus class="mr-2 size-4" />
				{generating ? 'Generating…' : 'Generate PNG'}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
