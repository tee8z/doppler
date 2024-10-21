import { writable, type Writable } from 'svelte/store';
interface SSELogListenerOptions {
	url: string;
	onMessage?: (message: string) => void;
	onError?: (error: Event) => void;
	onDisconnect?: () => void;
	onConnect?: () => void;
	maxRetries?: number;
	retryInterval?: number;
}

export function createSSELogListener(options: SSELogListenerOptions) {
	const logs: Writable<string[]> = writable([]);
	let eventSource: EventSource | null = null;
	let retryCount = 0;
	const maxRetries = options.maxRetries ?? 5;
	const retryInterval = options.retryInterval ?? 5000;

	function connect() {
		if (eventSource) {
			eventSource.close();
		}
		const sseUrl = options.url.startsWith('http')
			? options.url
			: new URL(options.url, window.location.origin).toString();

		eventSource = new EventSource(sseUrl);

		eventSource.onopen = () => {
			console.log('SSE connection opened');
			retryCount = 0;
			if (options.onConnect) {
				options.onConnect();
			}
		};

		eventSource.onmessage = (event) => {
			const rawMessage = event.data;
			processLogMessage(rawMessage);
		};

		eventSource.onerror = (error) => {
			console.error('SSE Error:', error);
			if (options.onError) {
				options.onError(error);
			}
			eventSource?.close();
			retryConnection();
		};
	}

	function retryConnection() {
		if (retryCount < maxRetries) {
			retryCount++;
			console.log(`Retrying connection (${retryCount}/${maxRetries})...`);
			setTimeout(connect, retryInterval);
		} else {
			console.error('Max retry attempts reached. Giving up.');
			if (options.onDisconnect) {
				options.onDisconnect();
			}
		}
	}

	function processLogMessage(logMessage: string) {
		logs.update((currentLogs: any) => [...currentLogs, logMessage]);
		if (options.onMessage) {
			options.onMessage(logMessage);
		}
	}

	function disconnect() {
		if (eventSource) {
			eventSource.close();
			eventSource = null;
			if (options.onDisconnect) {
				options.onDisconnect();
			}
		}
	}

	return {
		connect,
		disconnect,
		logs
	};
}
