export const toolbox = {
	kind: 'flyoutToolbox',
	contents: [
		{
			kind: 'block',
			type: 'loop'
		},
		{
			kind: 'block',
			type: 'loop_every'
		},
		{
			kind: 'block',
			type: 'bitcoind_miner'
		},
		{
			kind: 'block',
			type: 'bitcoind'
		},
		{
			kind: 'block',
			type: 'mine_blocks'
		},
		{
			kind: 'block',
			type: 'send_on_chain'
		},
		{
			kind: 'block',
			type: 'ln_pair'
		},
		{
			kind: 'block',
			type: 'open_channel'
		},
		{
			kind: 'block',
			type: 'close_channel'
		},
		{
			kind: 'block',
			type: 'send_ln'
		},

		{
			kind: 'block',
			type: 'visualizer'
		},
		{
			kind: 'block',
			type: 'up'
		},
		{
			kind: 'block',
			type: 'skip_conf'
		}
	]
};
