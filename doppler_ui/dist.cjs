//used in build process please do not touch
const { bin } = require('./package.json');
const execSync = require('child_process').execSync;
const path = require('path');
const fs = require('fs');
const ini = require('ini');
const { v7 } = require('uuid');

const configPath = './ui_config/server.conf.ini';

// Default configuration
const defaultConfig = {
	paths: {
		dopplerScriptsFolder: '~/.doppler/doppler_scripts',
		logsFolder: '~/.doppler/doppler_logs',
		scriptsFolder: '~/.doppler/scripts',
		dopplerBinaryPath: '~/.doppler/doppler',
		currentWorkingDirectory: '~/.doppler'
	}
};

let config;
try {
	config = ini.parse(fs.readFileSync(configPath, 'utf-8'));
} catch (error) {
	console.log('Config file not found. Creating a new one with default settings.');
	config = defaultConfig;
}

if (!config.paths) {
	config.paths = {};
}

config.paths = {
	...defaultConfig.paths,
	...config.paths
};

// Save the updated config
fs.writeFileSync(configPath, ini.stringify(config));

console.log('Configuration file updated and saved.');

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
						const uuid = v7();
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

// Run npm build
console.log('Running npm build...');
execSync('bun run build', { stdio: 'inherit' });

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

// Process Doppler scripts
processDopplerScripts();

const prodConf = './ui_config/server.conf.ini';
const directory = path.dirname(prodConf);
fs.mkdirSync(directory, { recursive: true });
fs.writeFileSync(prodConf, ini.stringify(config));
