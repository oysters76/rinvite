<script lang="ts">
	import { goto } from '$app/navigation';
	import { isAuthenticated } from '$lib/stores/session';

	// Landing: send to the dashboard or sign in.
	goto(isAuthenticated() ? '/events' : '/login', { replaceState: true });
</script>
