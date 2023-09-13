<script lang="ts">
	import type { IconName } from './icons';
	import icons from './icons';

	export let name: IconName;
</script>

<span class={` ${$$props.class || 'inline-flex h-6 text-primary'}`}>
	<!-- NOTE: if we don't know the source of the data, using @html can lead to XSS attacks.
  In this case, we know the data is safe because it's an SVG in our repo -->
	<!-- eslint-disable-next-line svelte/no-at-html-tags -->
	{@html icons[name]}
</span>