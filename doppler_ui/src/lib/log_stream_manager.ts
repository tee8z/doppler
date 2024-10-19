import fs from 'fs';

interface StreamInfo {
	stream: fs.WriteStream;
	lastUsed: number;
}

class LogStreamManager {
	private streams: Map<string, StreamInfo> = new Map();
	private cleanupInterval: NodeJS.Timeout | null = null;
	private static instance: LogStreamManager | null = null;

	private constructor() {}

	static getInstance(): LogStreamManager {
		if (!LogStreamManager.instance) {
			LogStreamManager.instance = new LogStreamManager();
		}
		return LogStreamManager.instance;
	}

	init() {
		if (!this.cleanupInterval) {
			this.cleanupInterval = setInterval(() => this.cleanup(), 5 * 60 * 1000); // 5 minutes
		}
	}

	getStream(logFilename: string): fs.WriteStream {
		const existing = this.streams.get(logFilename);
		if (existing) {
			console.log(`using exisiting stream: ${logFilename}`);
			existing.lastUsed = Date.now();
			return existing.stream;
		}

		const newStream = fs.createWriteStream(logFilename, { flags: 'a' });
		this.streams.set(logFilename, { stream: newStream, lastUsed: Date.now() });
		console.log(`new stream created: ${logFilename}`);
		newStream.on('error', (error: any) => {
			console.error(`Error with log stream for ${logFilename}: ${error.message}`);
			this.closeStream(logFilename);
		});

		return newStream;
	}

	private closeStream(logFilename: string) {
		const streamInfo = this.streams.get(logFilename);
		if (streamInfo) {
			streamInfo.stream.end();
			this.streams.delete(logFilename);
		}
	}

	private cleanup() {
		const now = Date.now();
		const MAX_INACTIVE_TIME = 30 * 60 * 1000; // 30 minutes
		for (const [logFilename, { lastUsed }] of this.streams.entries()) {
			if (now - lastUsed > MAX_INACTIVE_TIME) {
				console.log(`Closing inactive log stream: ${logFilename}`);
				this.closeStream(logFilename);
			}
		}
	}

	shutdown() {
		if (this.cleanupInterval) {
			clearInterval(this.cleanupInterval);
		}
		for (const [logFilename] of this.streams.entries()) {
			this.closeStream(logFilename);
		}
	}
}

export const logStreamManager = LogStreamManager.getInstance();
