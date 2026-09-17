#!/usr/bin/env node

const path = require('node:path');
const os = require('node:os');
const { spawn } = require('node:child_process');
const { readFile, writeFile, copyFile, rm } = require('node:fs/promises');

/**
 * Run and wait for command to complete.
 * @param {string[]} command
 * @param {SpawnOptionsWithStdioTuple} [opts]
 * @returns {Promise<void>}
 */
async function run(command, opts = undefined) {
  return new Promise((resolve, reject) => {
    console.log(`running: ${command.join(' ')}`);
    const child = spawn(command[0], command.slice(1), {
      stdio: ['ignore', 'inherit', 'inherit'],
      ...opts,
    });
    child.once('error', reject);
    child.once('close', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`process exit with code ${code}`));
      }
    });
  });
}

/**
 * @param {string} filepath
 * @param  {...([Buffer, Buffer])} binaryReplacements
 */
async function replaceBytes(filepath, ...binaryReplacements) {
  let content = await readFile(filepath);
  for (const [search, replace] of binaryReplacements) {
    let idx = -1;
    while ((idx = content.indexOf(search, idx + 1)) !== -1) {
      replace.copy(content, idx, 0, replace.length);
    }
  }
  await writeFile(filepath, content);
}

/**
 * @param {string} filepath
 * @param  {...([RegExp|string, string|(...args: string) => unknown])} replacementRules
 */
async function replaceFileByRegex(filepath, ...replacementRules) {
  let content = await readFile(filepath, 'utf-8');
  for (const [search, replace] of replacementRules) {
    content = content.replace(search, replace);
  }
  await writeFile(filepath, content, 'utf-8');
}

/**
 * @param {string | undefined} value
 * @param [defaultValue=false]
 * @return {boolean}
 */
function parseBoolean(value, defaultValue = false) {
  if (value === undefined || value === null) {
    return defaultValue;
  }

  if (Number.isFinite(+value)) {
    return Boolean(value);
  }

  return value.toLowerCase() === 'true';
}

async function main() {
  const wasmSourceDir = path.join(__dirname, '..', 'um_wasm');
  const wasmOutDir = path.resolve(__dirname, 'pkg');
  const wasmDistDir = path.resolve(__dirname, 'dist');
  const wasmRelOutDir = path.relative(wasmSourceDir, wasmOutDir);
  const profileFlag = parseBoolean(process.env.BUILD_RELEASE, true) ? '--release' : '--dev';

  if (process.env.BUILD_SKIP_WASM_PACK !== '1') {
    await rm(wasmOutDir, { recursive: true, force: true });
    await run(['wasm-pack', 'build', profileFlag, '--target', 'web', '--out-dir', wasmRelOutDir], {
      cwd: path.resolve(__dirname, '..', 'um_wasm'),
    });
  }

  // Remove unneeded files
  await Promise.all([
    ...['.gitignore', 'package.json', 'README.md'].map((name) => {
      return rm(path.join(wasmOutDir, name), { force: true });
    }),
    rm(wasmDistDir, { recursive: true, force: true }),
  ]);

  const homeDir = os.homedir();
  const dummyHome = '/h' + homeDir.slice(2).replace(/./g, '_');

  // Patch some files...
  await Promise.all([
    replaceFileByRegex(path.join(wasmOutDir, 'um_wasm.js'), [/export default (__wbg_init);/, 'export { $1 };']),
    replaceFileByRegex(path.join(wasmOutDir, 'um_wasm.d.ts'), [/export default (function __wbg_init)/, 'export $1']),
    replaceBytes(path.join(wasmOutDir, 'um_wasm_bg.wasm'), [
      Buffer.from(homeDir, 'utf-8'),
      Buffer.from(dummyHome, 'utf-8'),
    ]),
    copyFile(path.join(__dirname, '../LICENSE_APACHE'), 'LICENSE_APACHE'),
    copyFile(path.join(__dirname, '../LICENSE_MIT'), 'LICENSE_MIT'),
  ]);

  // Ask rollup to build bundles.
  await run(['pnpm', 'build:bundle']);
  await run(['pnpm', 'exec', 'prettier', '--ignore-path', '', '-w', 'dist/loader.d.ts']);
}

main()
  .catch((err) => {
    console.error(err);
    process.exit(1);
  })
  .then(() => {
    process.exit(0);
  });
