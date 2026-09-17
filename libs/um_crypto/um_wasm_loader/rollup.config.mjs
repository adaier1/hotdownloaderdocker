import { defineConfig } from 'rollup';
import { wasm } from '@rollup/plugin-wasm';
import replace from '@rollup/plugin-replace';
import { dts } from 'rollup-plugin-dts';
import { readFileSync } from 'node:fs';

const pkgJson = JSON.parse(readFileSync(new URL('package.json', import.meta.url), 'utf-8'));

function makePlugins({ sync }) {
  const plugins = [];
  plugins.push(
    wasm({
      sync: sync ? ['pkg/um_wasm_bg.wasm'] : [],
      fileName: 'um_wasm_bg.wasm',
    }),
  );
  plugins.push(
    replace({
      preventAssignment: true,
      values: {
        'process.env.UMC_INLINE_BUILD': JSON.stringify(String(sync ? 1 : 0)),
        'process.env.UMC_VERSION': JSON.stringify(pkgJson.version),
      },
    }),
  );
  return plugins;
}

export default defineConfig([
  {
    input: 'src/loader.mjs',
    output: {
      file: 'dist/loader.js',
      format: 'cjs',
    },
    plugins: [...makePlugins({ sync: false })],
  },
  {
    input: 'src/loader.mjs',
    output: {
      file: 'dist/loader.mjs',
      format: 'es',
    },
    plugins: [...makePlugins({ sync: false })],
  },
  {
    input: 'src/loader.mjs',
    output: {
      file: 'dist/loader-inline.js',
      format: 'cjs',
    },
    plugins: [...makePlugins({ sync: true })],
  },
  {
    input: './src/loader.d.ts',
    output: [{ file: 'dist/loader.d.ts', format: 'es' }],
    plugins: [dts()],
  },
]);
