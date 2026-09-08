// Used by the ignored Rust live file round-trip test. Credentials stay in env.
const assert = require('node:assert/strict');
const path = require('node:path');
const sdk = require(path.resolve(process.env.BOX_SDK_PATH));
const api = new sdk.BoxApi(new sdk.Configuration({
  accessToken: process.env.BOX_API_KEY,
  basePath: process.env.BOX_BASE_URL || 'https://ascii.dev/api/box/v1',
}));
(async () => {
  const boxId = process.env.BOX_ID;
  const filePath = process.env.BOX_TEST_PATH;
  const read = await api.readFile({ boxId, path: filePath, encoding: 'utf8' }, { signal: AbortSignal.timeout(60_000) });
  assert.equal(read.content, 'written-by-rust');
  await api.writeFile({ boxId, fileWriteRequest: {
    path: filePath, content: 'written-by-typescript', encoding: 'utf8',
  } }, { signal: AbortSignal.timeout(60_000) });
  console.log('PASS: SDK read Rust file and wrote replacement');
})().catch(error => {
  console.error(`SDK file round trip failed (HTTP ${error.response?.status || 'unknown'})`);
  process.exitCode = 1;
});
