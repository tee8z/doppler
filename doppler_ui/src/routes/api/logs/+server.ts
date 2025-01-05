import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import * as fs from 'fs';
import * as path from 'path';
import chokidar from 'chokidar';
import { parse } from 'ini';
import { UI_CONFIG_PATH } from '$env/static/private';

const configPath = UI_CONFIG_PATH || path.join(process.cwd(), '/build/ui_config');
const config = parse(fs.readFileSync(`${configPath}/server.conf.ini`, 'utf-8'));
const LOGS_FOLDER = config.paths.logsFolder;

// Since we want to tail the log file we need to manually handle watching the file for change and pushing
// it on the SSE stream, if we just used the raw file stream we would require a reconnection every time
// we come to the end of the file
export const GET: RequestHandler = async function (event) {
	console.log('Received GET request for log streaming');
	const id = event.url.searchParams.get('id');
	if (!id) {
		console.error('ID parameter is missing');
		return json({ error: 'ID parameter is required' }, { status: 400 });
	}

	const logFilename = path.join(LOGS_FOLDER, `${id}.log`);
	console.log(`Log file path: ${logFilename}`);

	if (!fs.existsSync(logFilename)) {
		console.error(`Log file not found: ${logFilename}`);
		return json({ error: 'Log file not found' }, { status: 404 });
	}
	let messageCounter = 0;

	const stream = new ReadableStream({
		start(controller) {
			console.debug('Starting ReadableStream');
			let position = 0;
			let isStreamClosed = false;

			const sendEvent = (data: string) => {
				if (isStreamClosed) {
					console.log('Stream is closed, cannot send more data');
					return;
				}
				try {
					messageCounter++;
					const message = `data: ${data}\n\n`;

					if (controller && controller.desiredSize !== null) {
						controller.enqueue(message);
					} else {
						console.log('Controller is not available or is closed');
						isStreamClosed = true;
					}
				} catch (e) {
					if (e instanceof TypeError && e.message.includes('Controller is already closed')) {
						console.log('Stream has been closed, stopping further sends');
						isStreamClosed = true;
					} else {
						console.warn('Unexpected error in sendEvent:', e);
					}
				}
			};

			const readLatestContent = () => {
				fs.stat(logFilename, (err, stats) => {
					if (err) {
						console.error(`Error getting file stats: ${err.message}`);
						sendEvent(`Error: ${err.message}`);
						return;
					}

					console.debug(`Current file size: ${stats.size}, Current position: ${position}`);

					if (stats.size < position) {
						console.warn('File size decreased, resetting position to 0');
						position = 0;
					}

					const stream = fs.createReadStream(logFilename, {
						start: position,
						encoding: 'utf8'
					});

					let buffer = '';
					stream.on('data', (chunk) => {
						buffer += chunk;
						const lines = buffer.split('\n');
						buffer = lines.pop() || '';

						lines.forEach((line) => {
							if (line.trim()) {
								sendEvent(line);
							}
						});

						position += Buffer.from(chunk).length;
					});

					stream.on('end', () => {
						if (buffer.trim()) {
							console.debug(`Sending final line: ${buffer}`);
							sendEvent(buffer);
						}
						console.debug('Finished reading current content');
					});

					stream.on('error', (streamErr) => {
						console.error(`Stream error: ${streamErr.message}`);
						sendEvent(`Error: ${streamErr.message}`);
					});
				});
			};

			// Initial read of the file
			readLatestContent();

			// Set up file watcher with increased polling interval
			const watcher = chokidar.watch(logFilename, {
				persistent: true,
				usePolling: true,
				interval: 100 // Increased from 5 to 100 milliseconds
			});

			watcher.on('change', (path) => {
				console.debug(`File ${path} has been changed`);
				readLatestContent();
			});

			// Clean up function
			return () => {
				console.debug('Cleaning up watcher');
				watcher.close();
			};
		},
		cancel() {
			console.debug('Stream cancelled by client');
		}
	});

	return new Response(stream, {
		headers: {
			'Content-Type': 'text/event-stream',
			'Cache-Control': 'no-cache',
			Connection: 'keep-alive'
		}
	});
};
