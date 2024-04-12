<script context="module" lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { derived, writable } from 'svelte/store';

	export const nodes = writable<any[]>([]);
	export const edges = writable<any[]>([]);
	const combined = derived([nodes, edges], ([$nodes, $edges]) => {
		console.log('Nodes updated:', $nodes);
		console.log('Edges updated:', $edges);
		return { cur_nodes: $nodes, cur_edges: $edges };
	});
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import * as d3 from 'd3';
	const dispatch = createEventDispatcher();

	let svg: any;
	let simulation: any;
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

		window.addEventListener('resize', () => resize(svg));

		combined.subscribe(({ cur_nodes, cur_edges }) => {
			if (cur_nodes.length == 0) {
				return;
			}
			if (!simulation) {
				console.log('render graph');
				renderGraph(svg, cur_nodes, cur_edges);
			} else if (svg) {
				console.log('update graph');
				updateGraph(svg, simulation, cur_nodes, cur_edges);
			}
		});
	});

	function renderGraph(svg: any, nodes: any, edges: any) {
		const svgElement = document.querySelector('#graphSVG');
		if (!svgElement) return;

		svg = d3
			.select(svgElement)
			.attr('width', '100%')
			.attr('height', '100%')
			.attr('viewBox', '0 0 1000 1000')
			.attr('preserveAspectRatio', 'xMidYMid meet')
			.append('g');

		window.addEventListener('resize', () => resize(svg));

		simulation = d3
			.forceSimulation(nodes)
			.force(
				'link',
				d3
					.forceLink(edges)
					.id((d: any) => d.id)
					.distance(300) //increase to greaten the length of the edges
			)
			.force('charge', d3.forceManyBody().strength(-900)) //make more negative to increase the space between non connected nodes
			.force('center', d3.forceCenter(400, 400));

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
			.attr('fill', '#b3b3cc');
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
			.style('stroke', (d: any) => {
				if (d.active) {
					return '#b3b3cc';
				} else {
					return '#ffa8a8';
				}
			});

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
			.attr('fill', '#FF9900')
			.attr('stroke', '#000000')
			.attr('stroke-width', '.3px')
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
			.attr('fill', '#FF9900')
			.attr('stroke', '#000000')
			.attr('stroke-width', '.3px')
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
			.style('fill', 'white')
			.attr('stroke', '#000000') // Set the outline color
			.attr('stroke-width', '.4px'); // Set the outline thickness

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
			// Adjust the scaling factor based on the total number of edges
			const totalEdges = sameSourceTargetPaths.length;
			const scalingFactor = 60 / totalEdges;
			dr = (dr * currentIndex) / scalingFactor;
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
				channel_id: data.channel_id,
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
			dispatch('dataEvent', identifiers);
		}
	}
	function updateGraph(svg: any, simulation: any, nodes: any, edges: any) {
		// Select all nodes
		const node = svg.selectAll('.node').data(nodes, (d: any) => d.id); // Assuming each node has a unique 'id' property

		// Handle exit selection
		node.exit().remove();

		// Handle enter selection
		const nodeEnter = node.enter().append('g').attr('class', 'node');
		nodeEnter
			.append('circle')
			.attr('r', 35)
			.attr('fill', '#15b2a7')
			.attr('stroke', 'grey')
			.attr('stroke-width', 2);
		nodeEnter
			.append('text')
			.attr('text-anchor', 'middle')
			.style('font-size', '24px')
			.style('font-family', 'sans-serif')
			.style('fill', 'white');

		// Merge enter and update selections
		const nodeMerge = nodeEnter.merge(node);

		// Update node positions and text
		nodeMerge
			.select('circle')
			.attr('cx', (d: any) => d.x)
			.attr('cy', (d: any) => d.y);
		nodeMerge.select('text').text((d: any) => d.alias);

		// Select all edges
		const path = svg.selectAll('path').data(edges, (d: any) => d.id); // Assuming each edge has a unique 'id' property

		// Handle exit selection
		path.exit().remove();

		// Handle enter selection
		const pathEnter = path
			.enter()
			.append('path')
			.attr('id', (d: any, i: any) => `path_${i}`)
			.attr('marker-end', (d: any) => `url(#initiator)`)
			.attr('stroke-width', 20)
			.attr('fill', 'none')
			.style('stroke', '#81fd90');

		// Merge enter and update selections
		const pathMerge = pathEnter.merge(path);

		// Update edge paths
		pathMerge.attr('d', (d: any) => linkArc(d, edges)); // Assuming linkArc is a function that calculates the path

		// Update edge labels
		const pathLabels = svg.selectAll('.path-label').data(edges, (d: any) => d.id);
		pathLabels.exit().remove();

		const pathLabelsEnter = pathLabels
			.enter()
			.append('text')
			.attr('class', 'path-label-local')
			.style('font-size', '22px')
			.attr('text-anchor', 'middle')
			.append('textPath')
			.attr('xlink:href', (d: any, i: any) => `#path_${i}`)
			.attr('startOffset', '30%')
			.attr('fill', '#c851e4')
			.text((d: any) => `${d.local_balance} sats`);

		const pathLabelsMerge = pathLabelsEnter.merge(pathLabels);

		// Update edge labels
		pathLabelsMerge.select('textPath').text((d: any) => `${d.local_balance} sats`);

		simulation.nodes(nodes);
		simulation.force('link').links(edges);
		simulation.alpha(1).restart();
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
