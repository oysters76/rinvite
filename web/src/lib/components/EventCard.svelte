<script lang="ts">
	import type { EventOverview } from '$lib/services';
	import { coupleTitle, fmtDate } from '$lib/format';

	let { event, onopen }: { event: EventOverview; onopen: () => void } = $props();
</script>

<button
	type="button"
	onclick={onopen}
	class="bg-card hover:bg-accent/40 flex cursor-pointer flex-col gap-3 border p-4 text-left transition-colors"
>
	<div class="kicker text-primary">{fmtDate(event.event_date)}</div>
	<div class="text-lg leading-tight font-extrabold">{coupleTitle(event)}</div>
	<div class="flex-1 text-[13px] opacity-80">{event.venue_name}</div>
	<div class="flex gap-5 border-t-2 pt-3">
		<div>
			<div class="text-xl font-semibold">{event.guestCount}</div>
			<div class="kicker">Guests</div>
		</div>
		<div>
			<div class="text-xl font-semibold">{event.einviteCount}</div>
			<div class="kicker">E-invite</div>
		</div>
		<div>
			<div class="text-xl font-semibold">{event.printCount}</div>
			<div class="kicker">Print</div>
		</div>
	</div>
</button>
