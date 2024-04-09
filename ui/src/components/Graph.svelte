<script lang="ts">
	import { onMount } from 'svelte';
	import * as d3 from 'd3';
	import { createEventDispatcher } from 'svelte';
	const dispatch = createEventDispatcher();

	export let nodes: any[] = [];
	export let edges: any[] = [];
	let svg: any;

	onMount(() => {
		const svgElement = document.querySelector('#graphSVG');
		if (!svgElement) return;

		const svg = d3
			.select(svgElement)
			.attr('width', '100%')
			.attr('height', '100%')
			.attr('viewBox', '0 0 1000 1000')
			.attr('preserveAspectRatio', 'xMidYMid meet')
			.append('g');

		renderGraph(svg, nodes, edges);

		window.addEventListener('resize', () => resize(svg));
	});

	function renderGraph(svg: any, nodes: any, edges: any) {
		const simulation = d3
			.forceSimulation(nodes)
			.force(
				'link',
				d3
					.forceLink(edges)
					.id((d: any) => d.id)
					.distance(400) //increase to greaten the length of the edges
			)
			.force('charge', d3.forceManyBody().strength(-200)) //make more negative to increase the space between non connected nodes
			.force('center', d3.forceCenter(500, 500));

		svg
			.append('defs')
			.selectAll('marker')
			.data(['initiator'])
			.enter()
			.append('marker')
			.attr('id', (d: any) => d)
			.attr('viewBox', '0 -5 10 10')
			.attr('refX', 15) //increase to move further up the line and away from the circle
			.attr('refY', 0)
			.attr('markerWidth', 8)
			.attr('markerHeight', 8)
			.attr('orient', 'auto')
			.append('path')
			.attr('d', 'M0,-5L10,0L0,5')
			.attr('fill', '#81fd90');

		const path = svg
			.append('g')
			.selectAll('path')
			.data(edges)
			.enter()
			.append('path')
			//.attr('class', (d: any) => `link ${d.type}`)
			.attr('marker-end', (d: any) => `url(#initiator)`)
			.attr('stroke-width', 5)
			.attr('fill', 'none')
			.style('stroke', '#81fd90');
		path.on('click', sendJson);
		svg
			.append('defs')
			.selectAll('marker')
			.data(['initiator'])
			.enter()
			.append('marker')
			.attr('id', function (d: any) {
				return d;
			})
			.attr('viewBox', '0 -5 10 10')
			.attr('refX', 0)
			.attr('refY', 0)
			.attr('markerWidth', 12)
			.attr('markerHeight', 12)
			.attr('orient', 'auto-start-reverse')
			.append('path')
			.attr('d', 'M0,-5L10,0L0,5');

		svg
			.selectAll('text.label')
			.data(edges)
			.enter()
			.append('text')
			.attr('class', 'label')
			.append('textPath')
			.attr('xlink:href', (d: any, i: any) => `#path_${i}`) // Reference the path by its unique ID
			.text((d: any) => d.capacity)
			.attr('startOffset', '50%')
			.style('text-anchor', 'middle')
			.attr('fill', 'purple');

		const node = svg.selectAll('.node').data(nodes).enter().append('g');
		node
			.append('circle')
			.attr('r', 35)
			.attr('fill', '#15b2a7')
			.attr('stroke', 'grey')
			.attr('stroke-width', 2);
		node
			.append('text')
			.text((d: any) => d.alias) // Assuming each node has an 'alias' property
			.attr('text-anchor', 'middle')
			.style('font-size', '24px')
			.style('font-family', 'sans-serif')
			.style('fill', 'white');

		node.on('click', sendJson);

		simulation.on('tick', () => {
			path.attr('d', linkArc);
			node.attr('transform', transform);
		});
	}

	function linkArc(d: any) {
		var dx = d.target.x - d.source.x,
			dy = d.target.y - d.source.y,
			dr = Math.sqrt(dx * dx + dy * dy);

		return (
			'M' +
			d.source.x +
			',' +
			d.source.y +
			'A' +
			dr +
			',' +
			dr +
			' 0 0,1 ' +
			d.target.x +
			',' +
			d.target.y
		);
	}

	function transform(d: any) {
		return `translate(${d.x},${d.y})`;
	}

	function resize(svg: any) {
		const svgElement = document.querySelector('#graphSVG');
		if (!svgElement) return;

		const svgWidth = svgElement.clientWidth;
		const svgHeight = svgElement.clientHeight;

		svg.attr('width', svgWidth).attr('height', svgHeight);
	}

	function sendJson(event: any, d: any) {
		const data: any = d3.select(event.target).data()[0];
		console.log(data);
		if (data.channel_id) {
			const identifiers = {
				channel_id: data.chan_id,
				source: data.source.id,
				type: 'channel',
				channel: data.channel
			};
			dispatch('dataEvent', identifiers);
		} else {
			const identifiers: any = {
				id: data.id,
				type: 'node',
				known: data.known
			};
			if (data.nodeInfo) {
				identifiers['nodeInfo'] = data.nodeInfo;
			}
			dispatch('dataEvent', identifiers);
		}
	}
</script>

<div class="graph-container">
	<svg id="graphSVG" bind:this={svg} />
</div>

<style>
	.graph-container {
		width: 100%;
		height: 100%;
		overflow: auto;
	}

	svg {
		width: 100%;
		height: 100%;
	}
</style>
