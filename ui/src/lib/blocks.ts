import Blockly from 'blockly';
import {
	bitcoind_miner,
	ln_pair,
	visualizer,
	up,
	open_channel,
	mine_blocks,
	skip_conf,
	close_channel,
	send_ln,
	bitcoind,
	send_on_chain,
	loop,
	loop_every
} from './definitions';

export function initBlocks() {
	Blockly.Blocks['bitcoind_miner'] = {
		init: function () {
			this.jsonInit(bitcoind_miner);

			this.setTooltip(() => {
				return 'Add a number to variable "%1".'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['ln_pair'] = {
		init: function () {
			this.jsonInit(ln_pair);

			this.setTooltip(() => {
				return 'Add a number to variable "%1".'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['visualizer'] = {
		init: function () {
			this.jsonInit(visualizer);

			this.setTooltip(() => {
				return 'Add a number to variable "%1".'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['up'] = {
		init: function () {
			this.jsonInit(up);

			this.setTooltip(() => {
				return 'Add a number to variable "%1".'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['open_channel'] = {
		init: function () {
			this.jsonInit(open_channel);
			this.setTooltip(() => {
				return 'Add a number to variable "%1".'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['mine_blocks'] = {
		init: function () {
			this.jsonInit(mine_blocks);

			this.setTooltip(() => {
				return 'Add a number to variable "%1".'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['skip_conf'] = {
		init: function () {
			this.jsonInit(skip_conf);

			this.setTooltip(() => {
				return 'Add a number to variable "%1".'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['close_channel'] = {
		init: function () {
			this.jsonInit(close_channel);

			this.setTooltip(() => {
				return 'Add a number to variable "%1".'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['send_ln'] = {
		init: function () {
			this.jsonInit(send_ln);

			this.setTooltip(() => {
				return 'Add a number to variable "%1".'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['bitcoind'] = {
		init: function () {
			this.jsonInit(bitcoind);

			this.setTooltip(() => {
				return 'Add a number to variable "%1".'.replace('%1', this.getFieldValue('VAR'));
			});
			this.setColour(230);
		}
	};

	Blockly.Blocks['send_on_chain'] = {
		init: function () {
			this.jsonInit(send_on_chain);

			this.setTooltip(() => {
				return 'Send on chain'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['loop'] = {
		init: function () {
			this.jsonInit(loop);

			this.setTooltip(() => {
				return 'Loop'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['loop_every'] = {
		init: function () {
			this.jsonInit(loop_every);

			this.setTooltip(() => {
				return 'Loop every'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};

	Blockly.Blocks['bitcoind_miner'] = {
		init: function () {
			this.jsonInit(bitcoind_miner);

			this.setTooltip(() => {
				return 'Mine blocks'.replace('%1', this.getFieldValue('VAR'));
			});
		}
	};
}
