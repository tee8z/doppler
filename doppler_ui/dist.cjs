//used in build process please do not touch
const { bin } = require('./package.json');
const execSync = require('child_process').execSync;
const path = require('path');
const fs = require('fs');
const crypto = require('crypto');

// Simple INI stringifier
function stringifyINI(obj) {
	let result = '';
	for (const [key, value] of Object.entries(obj)) {
		if (typeof value === 'object') {
			result += `[${key}]\n`;
			for (const [subKey, subValue] of Object.entries(value)) {
				result += `${subKey} = ${subValue}\n`;
			}
		} else {
			result += `${key} = ${value}\n`;
		}
	}
	return result;
}

function generateUUIDv4() {
	const bytes = crypto.randomBytes(16);

	// Set version (4) and variant (2) bits
	bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
	bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 2

	// Convert to hexadecimal string and insert hyphens
	return bytes
		.toString('hex')
		.match(/(.{8})(.{4})(.{4})(.{4})(.{12})/)
		.slice(1)
		.join('-');
}

// Existing code for bunTargets and platform detection...
const bunTargets = {
	'x86_64-pc-windows-msvc': 'bun-windows-x64',
	'aarch64-apple-darwin': 'bun-darwin-arm64',
	'x86_64-apple-darwin': 'bun-darwin-x64',
	'aarch64-unknown-linux-gnu': 'bun-linux-arm64',
	'x86_64-unknown-linux-gnu': 'bun-linux-x64'
};
const distTarget = process.env.CARGO_DIST_TARGET || process.env.DIST_TARGET;
if (!distTarget) {
	throw "DIST_TARGET isn't set, so we don't know what platform to build!";
}
const bunTarget = bunTargets[distTarget];
if (!bunTarget) {
	throw `To the best of our knowledge, bun does not support building for ${distTarget}`;
}
const binExt = distTarget.includes('windows') ? '.exe' : '';
const isDarwin = distTarget.includes('apple-darwin');
// Existing build process...
if (isDarwin) {
	console.log('Darwin detected. Installing @rollup/rollup-darwin-x64...');
	execSync('npm install @rollup/rollup-darwin-x64', { stdio: 'inherit' });
	console.log('Removing old installed dependencies ...');
	execSync('rm package-lock.json', { stdio: 'inherit' });
	execSync('rm -rf node_modules', { stdio: 'inherit' });
}

// Setup npm
console.log('Installing dependencies with npm...');
execSync('npm install', { stdio: 'inherit' });

// Setup bun
console.log('Installing dependencies with bun...');
execSync('bun install', { stdio: 'inherit' });

process.env.UI_CONFIG_PATH = '/ui_config';

// Run npm build
console.log('Running npm build...');
execSync('npm run build', {
	stdio: 'inherit',
	env: { ...process.env }
});

// Function to process Doppler scripts
function processDopplerScripts() {
	const repoRoot = path.resolve(__dirname, '..');
	const examplesDir = path.join(repoRoot, 'examples/doppler_files');
	const dopplerScriptsDir = path.join(repoRoot, 'doppler_scripts');

	if (fs.existsSync(dopplerScriptsDir)) {
		fs.rmSync(dopplerScriptsDir, { recursive: true, force: true });
	}

	fs.mkdirSync(dopplerScriptsDir, { recursive: true });

	function processDirectory(dir, relativePath = '') {
		const entries = fs.readdirSync(dir, { withFileTypes: true });
		for (const entry of entries) {
			const fullPath = path.join(dir, entry.name);
			const newRelativePath = path.join(relativePath, entry.name);

			if (entry.isDirectory()) {
				// Create corresponding directory in doppler_scripts
				const newDir = path.join(dopplerScriptsDir, newRelativePath);
				if (!fs.existsSync(newDir)) {
					fs.mkdirSync(newDir, { recursive: true });
				}
				processDirectory(fullPath, newRelativePath);
			} else if (entry.isFile()) {
				if (entry.name === 'README.md') {
					// Copy README.md without changing the name
					fs.copyFileSync(fullPath, path.join(dopplerScriptsDir, newRelativePath));
				} else if (entry.name.endsWith('.doppler')) {
					const existingFiles = fs.readdirSync(path.join(dopplerScriptsDir, relativePath));
					const baseFileName = path.parse(entry.name).name;
					const existingFile = existingFiles.find(
						(file) => file.startsWith(baseFileName) && file.endsWith('.doppler')
					);

					if (!existingFile) {
						// If no existing file, create a new one with UUID
						const uuid = generateUUIDv4();
						const newFileName = `${baseFileName}_${uuid}.doppler`;
						const newPath = path.join(dopplerScriptsDir, relativePath, newFileName);
						fs.copyFileSync(fullPath, newPath);
					} else {
						// If existing file, update its content
						const existingPath = path.join(dopplerScriptsDir, relativePath, existingFile);
						fs.copyFileSync(fullPath, existingPath);
					}
				}
			}
		}
	}

	processDirectory(examplesDir);
	console.log('Doppler scripts processed and copied to doppler_scripts directory.');
}

//* create a bin, wont work due to issues with two tools but wont build otherwise */
// for each binary, run bun
for (binName of Object.keys(bin)) {
	const binScript = bin[binName];
	const binPath = `${binName}${binExt}`;

	// Change to the directory of the script
	const scriptDir = path.dirname(binScript);
	process.chdir(scriptDir);

	// Get the relative path of the script from the new working directory
	const relativeScriptPath = path.basename(binScript);

	// Run bun build with the relative path
	execSync(
		`bun build ${relativeScriptPath} --compile --target ${bunTarget} --outfile ${path.join(
			'..',
			binPath
		)}`,
		{
			stdio: 'inherit'
		}
	);
}

console.log('Starting processing doppler example files');

// Process Doppler scripts
processDopplerScripts();

const configPath = './ui_config/server.conf.ini';

// Default configuration
const defaultConfig = {
	paths: {
		dopplerScriptsFolder: '$DEST_FOLDER/doppler_scripts',
		logsFolder: '$DEST_FOLDER/doppler_logs',
		scriptsFolder: '$DEST_FOLDER/scripts',
		dopplerBinaryPath: '$DEST_FOLDER/doppler',
		currentWorkingDirectory: '$DEST_FOLDER'
	}
};

fs.writeFileSync('package.json', '');

// Save the updated config
const directory = path.dirname(configPath);
fs.mkdirSync(directory, { recursive: true });
fs.writeFileSync(configPath, stringifyINI(defaultConfig));

console.log('Configuration file updated and saved.');
