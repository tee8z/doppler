<script lang="ts">
	export let type: 'button' | 'submit' = 'button';
</script>

<button {type} on:click>
	<slot />
</button>

<style lang="postcss">
	button {
		@apply p-2 bg-green-600 dark:bg-gray-700 dark:hover:bg-gray-500 text-white rounded hover:bg-green-400 transition-all;
	}
</style>
