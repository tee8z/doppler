import { Transform } from 'stream';

export function createLogParser() {
	return new Transform({
		transform(chunk: any, encoding: any, callback: any) {
			const line = chunk.toString().trim();
			const regex =
				/\[(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z)\s+\u001b\[(\d+)m([A-Z]+)\u001b\[0m\]/;
			const match = line.match(regex);
			if (match) {
				const [fullMatch, timestamp, colorCode, logLevel] = match;
				const parsed = line.replace(fullMatch, `[${logLevel}]`);
				this.push(parsed + '\n');
			} else {
				this.push(line + '\n');
			}
			callback();
		}
	});
}
