<script context="module" lang="ts">
	import { writable, derived } from 'svelte/store';

	export const nodes = writable<any[]>([]);
	export const edges = writable<any[]>([]);
	const combined = derived([nodes, edges], ([$nodes, $edges]) => {
		console.log('Nodes updated:', $nodes);
		console.log('Edges updated:', $edges);
		return { cur_nodes: $nodes, cur_edges: $edges };
	});
</script>

<script lang="ts">
	import { onMount, createEventDispatcher, afterUpdate } from 'svelte';
	import cytoscape from 'cytoscape';
	import coseBilkent from 'cytoscape-cose-bilkent';

	const dispatch = createEventDispatcher();
	let container: HTMLElement;
	let cy: cytoscape.Core;
	let debugMessage = '';
	let prevNodes: any[] = [];
	let prevEdges: any[] = [];

	afterUpdate(() => {
		console.log('Graph component updated');
		if (cy) {
			console.log('Resizing Cytoscape instance');
			cy.resize();
			centerGraph();
		} else {
			console.log('Cytoscape instance not available on update');
		}
	});

	onMount(() => {
		console.log('Graph component mounted');
		debugMessage = 'Mounting...';
		if (typeof cytoscape === 'undefined') {
			console.error('Cytoscape is not defined');
			debugMessage = 'Error: Cytoscape not found';
			return;
		}
		if (typeof coseBilkent === 'undefined') {
			console.error('coseBilkent layout is not defined');
			debugMessage = 'Error: coseBilkent not found';
			return;
		}
		cytoscape.use(coseBilkent);
		initCytoscape();

		combined.subscribe(({ cur_nodes, cur_edges }) => {
			if (cur_nodes.length === 0) return;
			updateGraph(cur_nodes, cur_edges);
		});

		cy.on('tap', 'node, edge', (evt: any) => {
			const element = evt.target;
			sendJson(element.data());
		});
	});

	function initCytoscape() {
		if (!container) {
			console.error('Container element not found');
			debugMessage = 'Error: Container not found';
			return;
		}
		console.log('Initializing Cytoscape');
		debugMessage = 'Initializing Cytoscape...';

		try {
			cy = cytoscape({
				container: container,
				style: [
					{
						selector: 'node',
						style: {
							'background-color': 'data(color)',
							label: 'data(alias)',
							'text-valign': 'center',
							'text-halign': 'center',
							color: 'white',
							'text-outline-color': '#000000',
							'text-outline-width': 0.4,
							'font-size': '24px',
							width: 70,
							height: 70
						}
					},
					{
						selector: 'edge',
						style: {
							width: 5,
							'line-color': 'data(color)',
							'target-arrow-color': 'data(color)',
							'target-arrow-shape': 'triangle',
							'curve-style': 'unbundled-bezier',
							'control-point-distances': function (ele: any) {
								const parallelEdges = ele.parallelEdges();
								const index = parallelEdges.indexOf(ele);

								const baseOffset = 80;
								if (parallelEdges.length === 1) {
									return [baseOffset]; // Default curve for single edges
								}
								// Logic for parallel edges
								return [baseOffset * (index - (parallelEdges.length - 1) / 2)];
							},
							'control-point-weights': [0.5],
							'edge-distances': 'intersection',
							'text-rotation': 'autorotate',
							'text-margin-y': -10,
							'font-size': '16px',
							'text-outline-width': 2,
							'text-background-opacity': 0,
							// Source label (Remote balance)
							'source-label': (ele: any) => `${ele.data('remote_balance')}`,
							'source-text-offset': 30,
							'source-text-margin-y': -10,
							'source-text-background-opacity': 1,
							'source-text-background-color': '#1a1a1a',
							'source-text-color': '#ffffff',
							'source-text-background-padding': 5,

							// Target label (Local balance)
							'target-label': (ele: any) => `${ele.data('local_balance')}`,
							'target-text-offset': 30,
							'target-text-margin-y': -10,
							'target-text-background-opacity': 1,
							'target-text-background-color': '#1a1a1a',
							'target-text-color': '#ffffff',
							'target-text-background-padding': 5,
							'text-wrap': 'wrap',
							color: 'white',
							'text-outline-color': '#FF9900'
						}
					}
				],
				layout: {
					name: 'cose-bilkent',
					animate: false,
					randomize: true,
					nodeDimensionsIncludeLabels: true,
					padding: 100,
					fit: true,
					componentSpacing: 300,
					nodeRepulsion: 12000,
					idealEdgeLength: 300,
					nodeOverlap: 20,
					preventOverlap: true,
					minNodeSpacing: 100,
					spacingFator: 1.6,
					// Add these parameters:
					initialEnergyOnIncremental: 0.5,
					gravity: 0.25,
					numIter: 2500
				},
				zoomingEnabled: false,
				userPanningEnabled: true
			});

			cy.on('layoutstop', function () {
				cy.edges().forEach((edge: any) => {
					// Force edge style update
					edge.style('control-point-distances', edge.style('control-point-distances'));
				});
			});

			console.log('Cytoscape initialized successfully');
			debugMessage = 'Cytoscape initialized';

			window.addEventListener('resize', resizeGraph);
		} catch (error: any) {
			console.error('Error initializing Cytoscape:', error);
			debugMessage = `Error initializing Cytoscape: ${error.message}`;
		}
	}

	function updateGraph(nodes: any[], edges: any[]) {
		console.log('Updating graph with nodes:', nodes.length, 'and edges:', edges.length);
		debugMessage = `Updating graph: ${nodes.length} nodes, ${edges.length} edges`;

		if (!cy) {
			console.error('Cytoscape instance not available for update');
			debugMessage = 'Error: Cytoscape not initialized';
			return;
		}

		const nodeChanges = getElementChanges(prevNodes, nodes, 'id');
		const edgeChanges = getElementChanges(prevEdges, edges, 'channel_id');

		// Remove nodes and edges that no longer exist
		nodeChanges.removed.forEach((node) => {
			cy.getElementById(node.id).remove();
		});
		edgeChanges.removed.forEach((edge) => {
			cy.getElementById(edge.channel_id).remove();
		});

		// Add new nodes and edges
		nodeChanges.added.forEach((node) => {
			cy.add({
				group: 'nodes',
				data: {
					...node,
					id: node.id,
					color: getNodeColor(node.type)
				}
			});
		});
		edgeChanges.added.forEach((edge) => {
			if (cy.getElementById(edge.source).length && cy.getElementById(edge.target).length) {
				cy.add({
					group: 'edges',
					data: {
						...edge,
						id: edge.channel_id,
						source: edge.source,
						target: edge.target,
						color: edge.active ? '#b3b3cc' : '#ffa8a8'
					}
				});
			}
		});

		// Update changed nodes and edges
		nodeChanges.changed.forEach((node) => {
			cy.getElementById(node.id).data(node);
		});
		edgeChanges.changed.forEach((edge) => {
			console.log(
				'Updating edge:',
				edge.channel_id,
				'Remote balance:',
				edge.remote_balance,
				'Local balance:',
				edge.local_balance,
				'Active:',
				edge.active
			);
			const cyEdge = cy.getElementById(edge.channel_id);
			// Update both the edge data and its color
			cyEdge.data(edge);
			cyEdge.style('line-color', edge.active ? '#b3b3cc' : '#ffa8a8');
			cyEdge.style('target-arrow-color', edge.active ? '#b3b3cc' : '#ffa8a8');
		});
		prevNodes = nodes;
		prevEdges = edges;

		if (
			nodeChanges.added.length > 0 ||
			nodeChanges.removed.length > 0 ||
			edgeChanges.added.length > 0 ||
			edgeChanges.removed.length > 0
		) {
			runLayout();
		} else {
			centerGraph();
		}
	}

	function getElementChanges(prevElements: any[], newElements: any[], idField: string) {
		const prevMap = new Map(prevElements.map((el) => [el[idField], el]));
		const newMap = new Map(newElements.map((el) => [el[idField], el]));

		const added = newElements.filter((el) => !prevMap.has(el[idField]));
		const removed = prevElements.filter((el) => !newMap.has(el[idField]));
		const changed = newElements.filter((el) => {
			const prevEl = prevMap.get(el[idField]);
			return prevEl && JSON.stringify(prevEl) !== JSON.stringify(el);
		});

		return { added, removed, changed };
	}

	function runLayout() {
		if (!cy) {
			console.error('Cytoscape instance not available for layout');
			debugMessage = 'Error: Cannot run layout';
			return;
		}
		console.log('Running layout');
		debugMessage = 'Running layout...';
		cy.layout({
			name: 'cose-bilkent',
			animate: false,
			randomize: false,
			nodeDimensionsIncludeLabels: false,
			padding: 50,
			fit: true,
			componentSpacing: 200,
			nodeRepulsion: 8000,
			idealEdgeLength: 200
		}).run();
		centerGraph();
		debugMessage = 'Layout complete';
	}

	function centerGraph() {
		if (cy) {
			cy.center();
			cy.fit();
		}
	}

	function resizeGraph() {
		if (cy) {
			console.log('Resizing graph');
			debugMessage = 'Resizing graph...';
			cy.resize();
			centerGraph();
		} else {
			console.error('Cannot resize: Cytoscape instance not available');
			debugMessage = 'Error: Cannot resize';
		}
	}

	function getNodeColor(type: string): string {
		switch (type) {
			case 'lnd':
				return '#15b2a7';
			case 'coreln':
				return '#9fca3c';
			case 'eclair':
				return '#8a2be2';
			default:
				return '#cccccc';
		}
	}

	function sendJson(data: any) {
		if (data.channel_id) {
			const identifiers = {
				channel_id: data.channel_id,
				source: data.source,
				type: 'channel',
				channel: data.data
			};
			dispatch('dataEvent', identifiers);
		} else {
			const identifiers = {
				id: data.id,
				type: 'node',
				known: data.known
			};
			dispatch('dataEvent', identifiers);
		}
	}
</script>

<div class="graph-container" bind:this={container}>
	{#if !cy}
		<p class="debug-message">Debug: {debugMessage}</p>
	{/if}
</div>

<style>
	.graph-container {
		width: 100%;
		height: 100%;
		display: flex;
		justify-content: center;
		align-items: center;
		overflow: hidden;
		position: relative;
	}
</style>
