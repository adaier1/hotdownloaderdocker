const { NCMFile, ready } = require('./dist/loader.js');

ready.then(() => {
  const ncm = new NCMFile();
  let n = ncm.open(new Uint8Array([]));
  console.assert(n !== 0, 'n should not be 0', { n });
  console.log('cjs test ok');
});
