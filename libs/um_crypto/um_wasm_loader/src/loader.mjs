import umWasm from '../pkg/um_wasm_bg.wasm';
import { __wbg_init, initPanicHook, initSync } from '../pkg/um_wasm.js';

export * from '../pkg/um_wasm.js';

function loader() {
  if (process.env.UMC_INLINE_BUILD === '1') {
    initSync({ module: umWasm() });
    initPanicHook();
    return Promise.resolve(true);
  } else {
    const url = new URL('um_wasm_bg.wasm', import.meta.url);
    const wasm =
      url.protocol === 'file:'
        ? import(/* @vite-ignore */ 'node:f' + 's/promises')
            .then((fs) => fs.readFile(url))
            .catch((err) => {
              console.log('read wasm failed', err);
            })
        : undefined;
    return __wbg_init({ module_or_path: wasm }).then(() => (initPanicHook(), true));
  }
}

export function getUmcVersion() {
  return process.env.UMC_VERSION;
}

export const ready = loader();
