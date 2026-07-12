<script lang="ts">
	import type { RsvpStatus } from '$lib/api';
	import Tag from './Tag.svelte';

	let { status }: { status: RsvpStatus } = $props();

	const label: Record<RsvpStatus, string> = {
		pending: 'Pending',
		attending: 'Attending',
		declined: 'Declined'
	};
	const variant = (s: RsvpStatus) =>
		s === 'attending' ? 'accent' : s === 'declined' ? 'outline' : 'neutral';
</script>

<Tag variant={variant(status)}>{label[status]}</Tag>
