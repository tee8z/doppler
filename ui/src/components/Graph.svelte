<script lang="ts">
	import { onMount } from 'svelte';
	import * as d3 from 'd3';
	import { createEventDispatcher } from 'svelte';
	const dispatch = createEventDispatcher();

	export let nodes: any[] = [];
	export let edges: any[] = [];
	let svg: any;
	onMount(() => {
		console.log('on mount');
		const svgElement = document.querySelector('#graphSVG');
		if (!svgElement) return;

		svg = d3
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
		const pathWidth = 5;
		const path = svg
			.append('g')
			.selectAll('path')
			.data(edges)
			.enter()
			.append('path')
			.attr('id', (d: any, i: any) => `path_${i}`)
			.attr('marker-end', (d: any) => `url(#initiator)`)
			.attr('stroke-width', pathWidth)
			.attr('fill', 'none')
			.style('stroke', '#81fd90');

		const pathLabels = svg
			.selectAll('.path-label')
			.data(edges)
			.enter()
			.append('text')
			.attr('class', 'path-label-local')
			.style('font-size', '22px')
			.attr('text-anchor', 'middle') // Adjust text-anchor as needed
			.append('textPath')
			.attr('xlink:href', (d: any, i: any) => `#path_${i}`) // Reference the path by its ID
			.attr('startOffset', '30%')
			.attr('fill', '#c851e4')
			.text((d: any) => `${d.local_balance} sats`);

		const pathLabelsEnd = svg
			.selectAll('.path-label-end')
			.data(edges)
			.enter()
			.append('text')
			.attr('class', 'path-label-remote')
			.style('font-size', '22px')
			.attr('text-anchor', 'middle') // Center the text horizontally
			.append('textPath')
			.attr('xlink:href', (d: any, i: any) => `#path_${i}`) // Reference the path by its ID
			.attr('startOffset', '70%') // Position the text at the end of the path
			.attr('fill', '#c851e4')
			.text((d: any) => `${d.remote_balance} sats`);

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
			path.attr('d', (d: any) => linkArc(d, edges)); // Pass the links array to linkArc
			node.attr('transform', transform);
		});
	}

	function linkArc(d: any, edges: any) {
		const dx = d.target.x - d.source.x,
			dy = d.target.y - d.source.y;
		let dr = Math.sqrt(dx * dx + dy * dy);

		const sameSourceTargetPaths = edges.filter(function (edge: any) {
			return edge.source === d.source && edge.target === d.target;
		});
		const currentIndex = sameSourceTargetPaths.indexOf(d);
		if (currentIndex > 0) {
			//This bit of math allows none of the curves to overlap
			dr = (dr * currentIndex) / 10;
		}
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
