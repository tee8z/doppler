<script lang="ts">
	export let type: 'button' | 'submit' = 'button';
</script>

<button {type} on:click>
	<slot />
</button>

<style lang="postcss">
	button {
		@apply p-4 mt-4 bg-green-600 text-white rounded hover:bg-green-400 transition-all;
	}
</style>