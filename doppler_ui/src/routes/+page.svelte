<script>
	import { browser } from '$app/environment';
	import DopplerLayout from '../components/DopplerLayout.svelte';
</script>

{#if browser}
	<DopplerLayout />
{/if}
