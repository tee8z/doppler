interface Node {
	id: string;
	alias: string;
	known: string;
	type?: string;
}

interface ChannelData {
	type: string;
	channel: any;
	perspective: 'source' | 'target';
}

interface Channel {
	source: string;
	target: string;
	channel_id: string;
	capacity: number;
	local_balance: number;
	remote_balance: number;
	initiator: boolean;
	active: boolean;
	data: ChannelData[];
}

interface NodeConnection {
	pubkey: string;
	alias: string;
	connection: any;
}

interface Nodes {
	[key: string]: any;
}

export class ChannelMapper {
	private nodeMap: Map<string, Node>;
	private channelMap: Map<string, Channel>;
	private uniqueNodes: Set<string>;
	private uniqueChannels: Set<string>;
	private nodeConnections: NodeConnection[];
	private nodes: Node[];
	private edges: Channel[];

	constructor() {
		this.nodeMap = new Map();
		this.channelMap = new Map();
		this.uniqueNodes = new Set();
		this.uniqueChannels = new Set();
		this.nodeConnections = [];
		this.nodes = [];
		this.edges = [];
	}

	public processNodeData(nodeData: Nodes, nodeConnections: NodeConnection[]) {
		this.clear();
		this.nodeConnections = nodeConnections;

		// Map all known nodes first
		this.mapAllNodes(nodeData);

		// Then map all channels and discover additional nodes
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (!value.online) return;
			this.mapChannels(value.type, key, value);
		});

		return {
			cur_nodes: this.nodes,
			cur_edges: this.edges
		};
	}

	private clear() {
		this.nodeMap.clear();
		this.channelMap.clear();
		this.uniqueNodes.clear();
		this.uniqueChannels.clear();
		this.nodes = [];
		this.edges = [];
	}

	private addNode(nodeId: string, alias: string, type?: string) {
		if (!this.nodeMap.has(nodeId)) {
			// First try to find a known connection
			const known = this.nodeConnections.find((node) => node.pubkey === nodeId);

			const nodeInfo: Node = {
				id: nodeId,
				alias: known?.alias || alias, // Use known alias if available, otherwise use provided alias
				known: nodeId
			};

			if (type) {
				nodeInfo.type = type;
			}

			this.nodeMap.set(nodeId, nodeInfo);
			this.nodes.push(nodeInfo);
			this.uniqueNodes.add(nodeId);
		} else if (type) {
			// If node exists and type is provided, update the type
			const existingNode = this.nodeMap.get(nodeId)!;
			existingNode.type = type;
		}
	}

	private mapAllNodes(nodeData: Nodes) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (!value.online) return;

			const nodeId = this.getNodeId(value);
			if (!nodeId) return;

			this.addNode(nodeId, key, value.type);
		});
	}

	private getNodeId(node: any): string | null {
		switch (node.type) {
			case 'lnd':
				return node.info?.identity_pubkey || null;
			case 'coreln':
				return node.info?.id || null;
			case 'eclair':
				return node.info?.nodeId || null;
			default:
				return null;
		}
	}

	private getChannelPoint(type: string, channel: any): string | null {
		switch (type) {
			case 'lnd':
				return channel.channel_point;
			case 'coreln': {
				// coreln channels have either channel_point or funding_txid + funding_output
				if (channel.channel_point) {
					return channel.channel_point;
				}
				if (channel.funding?.funding_txid || channel.funding_txid) {
					const txid = channel.funding?.funding_txid || channel.funding_txid;
					const output = channel.funding?.funding_output ?? channel.funding_output;
					if (txid && output !== undefined) {
						return `${txid}:${output}`;
					}
				}
				// Try getting from fundingTx if available
				if (channel.funding_txid) {
					return `${channel.funding_txid}:${channel.funding_output || 0}`;
				}
				return null;
			}
			case 'eclair': {
				const outPoint = channel.data?.commitments?.active[0]?.fundingTx?.outPoint;
				return outPoint || null;
			}
			default:
				return null;
		}
	}

	private mapChannels(type: string, nodeKey: string, nodeData: any) {
		const currentPubkey = this.getNodeId(nodeData);
		if (!currentPubkey) return;

		const channels = this.getChannels(type, nodeData);
		if (!channels) return;

		channels.forEach((channel: any) => {
			const channelPoint = this.getChannelPoint(type, channel);
			if (!channelPoint) return;

			const remotePubkey = this.getRemotePubkey(type, channel);
			if (!remotePubkey) return;

			// Add remote node if we haven't seen it before
			this.addNode(remotePubkey, remotePubkey);

			// Determine channel properties
			const isInitiator = this.isChannelInitiator(type, channel);

			// Create data entry for this node's view of the channel
			const channelData: ChannelData = {
				type,
				channel,
				perspective: isInitiator ? 'source' : 'target'
			};

			if (!this.channelMap.has(channelPoint)) {
				// Creating new channel
				const channelInfo: Channel = {
					source: isInitiator ? currentPubkey : remotePubkey,
					target: isInitiator ? remotePubkey : currentPubkey,
					channel_id: channelPoint,
					capacity: this.getChannelCapacity(type, channel),
					local_balance: this.getLocalBalance(type, channel),
					remote_balance: this.getRemoteBalance(type, channel),
					initiator: isInitiator,
					active: this.isChannelActive(type, channel),
					data: [channelData] // Start with this node's view
				};
				this.channelMap.set(channelPoint, channelInfo);
				this.edges.push(channelInfo);
				this.uniqueChannels.add(channelPoint);
			} else {
				// Channel exists, add this node's view if we don't have it
				const existingChannel = this.channelMap.get(channelPoint)!;

				// Check if we already have data from this node
				const hasNodeData = existingChannel.data.some(
					(d) =>
						d.type === type &&
						((d.perspective === 'source' && existingChannel.source === currentPubkey) ||
							(d.perspective === 'target' && existingChannel.target === currentPubkey))
				);

				// Only add if we don't have this node's view yet
				if (!hasNodeData) {
					existingChannel.data.push(channelData);
				}

				// Update channel properties if we're the initiator
				if (isInitiator) {
					existingChannel.capacity = this.getChannelCapacity(type, channel);
					existingChannel.local_balance = this.getLocalBalance(type, channel);
					existingChannel.remote_balance = this.getRemoteBalance(type, channel);
					existingChannel.active = this.isChannelActive(type, channel);
				}
			}
		});
	}

	private getChannels(type: string, nodeData: any): any[] | null {
		return nodeData.channels || null;
	}

	private getRemotePubkey(type: string, channel: any): string | null {
		switch (type) {
			case 'lnd':
				return channel.remote_pubkey;
			case 'coreln':
				return channel.peer_id;
			case 'eclair':
				return channel.nodeId;
			default:
				return null;
		}
	}

	private isChannelInitiator(type: string, channel: any): boolean {
		switch (type) {
			case 'lnd':
				return channel.initiator;
			case 'coreln':
				return channel.opener === 'local';
			case 'eclair':
				return channel.data?.commitments?.params?.localParams?.isInitiator || false;
			default:
				return false;
		}
	}

	private getChannelCapacity(type: string, channel: any): number {
		switch (type) {
			case 'lnd':
				return parseInt(channel.capacity) || 0;
			case 'coreln':
				return channel.amount_msat ? Math.floor(channel.amount_msat / 1000) : 0;
			case 'eclair':
				return channel.data?.commitments?.active[0]?.fundingTx?.amountSatoshis || 0;
			default:
				return 0;
		}
	}

	private getLocalBalance(type: string, channel: any): number {
		switch (type) {
			case 'lnd':
				return parseInt(channel.local_balance) || 0;
			case 'coreln':
				return channel.our_amount_msat ? Math.floor(channel.our_amount_msat / 1000) : 0;
			case 'eclair':
				return Math.floor(
					(channel.data?.commitments?.active[0]?.localCommit?.spec?.toLocal || 0) / 1000
				);
			default:
				return 0;
		}
	}

	private getRemoteBalance(type: string, channel: any): number {
		switch (type) {
			case 'lnd':
				return parseInt(channel.remote_balance) || 0;
			case 'coreln': {
				const total = channel.amount_msat ? Math.floor(channel.amount_msat / 1000) : 0;
				const local = channel.our_amount_msat ? Math.floor(channel.our_amount_msat / 1000) : 0;
				return total - local;
			}
			case 'eclair':
				return Math.floor(
					(channel.data?.commitments?.active[0]?.localCommit?.spec?.toRemote || 0) / 1000
				);
			default:
				return 0;
		}
	}

	private isChannelActive(type: string, channel: any): boolean {
		switch (type) {
			case 'lnd':
				return channel.active;
			case 'coreln':
				return channel.state === 'CHANNELD_NORMAL';
			case 'eclair':
				return channel.state === 'NORMAL';
			default:
				return false;
		}
	}
}
