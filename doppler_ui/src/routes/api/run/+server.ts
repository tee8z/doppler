import { json, type RequestHandler } from '@sveltejs/kit';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { parse } from 'ini';
import { createLogParser } from '$lib/log_transformers';
import { logStreamManager } from '$lib/log_stream_manager';

const configPath = process.env.UI_CONFIG_PATH || path.join(process.cwd(), '/build/ui_config');
const config = parse(fs.readFileSync(`${configPath}/server.conf.ini`, 'utf-8'));

const DOPPLER_SCRIPTS_FOLDER = config.paths.dopplerScriptsFolder;
const LOGS_FOLDER = config.paths.logsFolder;

interface ScriptPayload {
	id: string;
	fullPath: string;
}

logStreamManager.init();

export const POST: RequestHandler = async function (event: any) {
	try {
		const body: ScriptPayload = await event.request.json();
		const { id, fullPath } = body;

		if (!id || !fullPath) {
			return json({ error: 'Invalid input. Both id and fullPath are required.' }, { status: 400 });
		}

		[DOPPLER_SCRIPTS_FOLDER, LOGS_FOLDER].forEach((folder) => {
			if (!fs.existsSync(folder)) {
				fs.mkdirSync(folder, { recursive: true });
			}
		});

		const scriptFilename = path.join(DOPPLER_SCRIPTS_FOLDER, fullPath);
		if (!fs.existsSync(scriptFilename)) {
			return json({ error: 'Script not found' }, { status: 404 });
		}
		console.log(`scriptFilename: ${scriptFilename}`);

		const logFilename = path.join(LOGS_FOLDER, `${id}.log`);
		console.log(`logFilename: ${scriptFilename}`);

		const logStream = logStreamManager.getStream(logFilename);

		console.log(`Attempting to spawn process for script: ${scriptFilename}`);
		const process = spawn(
			config.paths.dopplerBinaryPath,
			[
				'-f',
				scriptFilename,
				'-r',
				'--ui-config-path',
				`${configPath}/info.conf.ini`,
				'--level',
				'debug'
			],
			{
				cwd: config.paths.currentWorkingDirectory,
				detached: true,
				stdio: ['ignore', 'pipe', 'pipe']
			}
		);

		if (!process.pid) {
			console.error('Failed to spawn process');
			return json({ error: 'Failed to spawn process' }, { status: 500 });
		}

		console.log(`Process spawned with PID: ${process.pid}`);

		process.unref();

		let stdoutData = '';
		let stderrData = '';

		process.stdout.on('data', (data: any) => {
			stdoutData += data;
			console.log(`Received stdout data: ${data.length} bytes`);
		});

		process.stderr.on('data', (data: any) => {
			stderrData += data;
			console.error(`Received stderr data: ${data.length} bytes`);
		});

		const stdoutLogParser = createLogParser();
		const stderrLogParser = createLogParser();

		process.stdout.pipe(stdoutLogParser).pipe(logStream);
		process.stderr.pipe(stderrLogParser).pipe(logStream);

		process.on('error', (error: any) => {
			const errorMessage = `Error with process: ${error.message}\n`;
			console.error(errorMessage);
			logStream.write(errorMessage);
		});

		process.on('close', (code: any) => {
			const closeMessage = `Child process exited with code ${code}\n`;
			console.log(closeMessage);
			logStream.write(closeMessage);

			console.log(`Total stdout: ${stdoutData.length} bytes`);
			console.log(`Total stderr: ${stderrData.length} bytes`);

			// Don't end the stream here, as it might be reused
			// logStream.end();
		});

		return json({
			message: 'Script execution started in the background',
			scriptPath: scriptFilename,
			logPath: logFilename,
			pid: process.pid
		});
	} catch (error) {
		console.error('Error:', error);
		return json(
			{
				error: 'An error occurred while processing the request'
			},
			{ status: 500 }
		);
	}
};
