export const bitcoind_miner = {
	type: 'bitcoind_miner',
	message0: 'BITCOIND_MINER %1 %2 s',
	args0: [
		{
			type: 'field_input',
			name: 'BITCOIND_NAME',
			text: 'bd1'
		},
		{
			type: 'field_number',
			name: 'BLOCK_TIME',
			value: 0,
			min: 1,
			precision: 1
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#F7931A',
	tooltip: '',
	helpUrl: ''
};

export const mine_blocks = {
	type: 'mine_blocks',
	message0: '%1 MINE_BLOCKS %2',
	args0: [
		{
			type: 'field_input',
			name: 'BITCOIND_NAME',
			text: 'bd1'
		},
		{
			type: 'field_number',
			name: 'NAME',
			value: 0,
			min: 0,
			precision: 1
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#F7931A',
	tooltip: '',
	helpUrl: ''
};

export const ln_pair = {
	type: 'ln_pair',
	message0: '%1 %2 PAIR %3',
	args0: [
		{
			type: 'field_dropdown',
			name: 'NODE_TYPE',
			options: [
				['LND', 'LND'],
				['CoreLN', 'CORELN'],
				['Eclair', 'ECLAIR']
			]
		},
		{
			type: 'field_input',
			name: 'NODE_NAME',
			text: 'lnd1'
		},
		{
			type: 'field_input',
			name: 'BITCOIND_NAME',
			text: 'bd1'
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#6731A4',
	tooltip: '',
	helpUrl: ''
};

export const visualizer = {
	type: 'visualizer',
	message0: 'VISUALIZER %1',
	args0: [
		{
			type: 'field_input',
			name: 'CMD',
			text: 'view'
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#71AF90',
	tooltip: '',
	helpUrl: ''
};

export const up = {
	type: 'up',
	message0: 'UP',
	previousStatement: null,
	nextStatement: null,
	colour: '#71AF90',
	tooltip: '',
	helpUrl: ''
};

export const open_channel = {
	type: 'open_channel',
	message0: '%1 OPEN_CHANNEL %2 AMT %3',
	args0: [
		{
			type: 'field_input',
			name: 'NODE_NAME',
			text: 'lnd1'
		},
		{
			type: 'field_input',
			name: 'CHANNEL_PARTNER',
			text: 'lnd2'
		},
		{
			type: 'field_input',
			name: 'CHANNEL_SIZE',
			text: '500000'
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#6731A4',
	tooltip: '',
	helpUrl: ''
};

export const skip_conf = {
	type: 'skip_conf',
	message0: 'SKIP_CONF',
	previousStatement: null,
	nextStatement: null,
	colour: '#71AF90',
	tooltip: '',
	helpUrl: ''
};

export const close_channel = {
	type: 'close_channel',
	message0: '%1 CLOSE_CHANNEL %2',
	args0: [
		{
			type: 'field_input',
			name: 'NODE_NAME',
			text: 'lnd1'
		},
		{
			type: 'field_input',
			name: 'CHANNEL_PARTNER',
			text: 'lnd2'
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#6731A4',
	tooltip: '',
	helpUrl: ''
};

export const send_ln = {
	type: 'send_ln',
	message0: '%1 SEND_LN %2 AMT %3',
	args0: [
		{
			type: 'field_input',
			name: 'NODE_NAME',
			text: 'lnd1'
		},
		{
			type: 'field_input',
			name: 'CHANNEL_PARTNER',
			text: 'lnd2'
		},
		{
			type: 'field_number',
			name: 'AMT',
			value: 1,
			min: 1,
			precision: 1
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#6731A4',
	tooltip: '',
	helpUrl: ''
};

export const bitcoind = {
	type: 'bitcoind',
	message0: 'BITCOIND %1',
	args0: [
		{
			type: 'field_input',
			name: 'BITCOIND_NAME',
			text: 'bd1'
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#F7931A',
	tooltip: '',
	helpUrl: ''
};

export const send_on_chain = {
	type: 'send_on_chain',
	message0: '%1 SEND_ON_CHAIN %2 AMT %3',
	args0: [
		{
			type: 'field_input',
			name: 'NODE_NAME',
			text: 'lnd1'
		},
		{
			type: 'field_input',
			name: 'PEER',
			text: 'lnd2'
		},
		{
			type: 'field_number',
			name: 'AMT',
			value: 1,
			min: 1,
			precision: 1
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#F7931A',
	tooltip: '',
	helpUrl: ''
};

export const loop = {
	type: 'loop',
	message0: 'LOOP %1 %2',
	args0: [
		{
			type: 'field_number',
			name: 'TIMES',
			value: 0,
			min: 0,
			precision: 1
		},
		{
			type: 'input_statement',
			name: 'NAME'
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#E87779',
	tooltip: '',
	helpUrl: ''
};

export const loop_every = {
	type: 'loop_every',
	message0: 'LOOP EVERY %1 %2',
	args0: [
		{
			type: 'field_number',
			name: 'TIMES',
			value: 0,
			min: 0,
			precision: 1
		},
		{
			type: 'input_statement',
			name: 'NAME'
		}
	],
	previousStatement: null,
	nextStatement: null,
	colour: '#E87779',
	tooltip: '',
	helpUrl: ''
};
