#!/usr/bin/env node
// 汽水远程签名服务：把本机客户端的 bdms.node 暴露成一个 HTTP 签名接口，
// 供 Docker / 服务器上的 HotDownloader 调用（容器里跑不了 Windows 原生组件）。
//
//   GET  /config  -> {"device_id":"…","identity":{"device_id":"…","iid":"…","fp":"…"}}
//   POST /sign    -> {"ok":true,"headers":{"X-Helios":"…","X-Medusa":"…"}}
//   入参：{"url":"…","method":"POST","body":"…","ts_ms":0,"headers":{…}}
//
// 环境变量：
//   QISHUI_SIGNER_PORT    监听端口，默认 8899
//   QISHUI_SIGNER_HOST    监听地址，默认 0.0.0.0（容器要访问宿主机，不能只听 127.0.0.1）
//   QISHUI_SIGNER_TOKEN   非空时要求 Authorization: Bearer <token>
//   QISHUI_BDMS_PATH      直接指定 bdms.node
//   QISHUI_CLIENT_DIR     汽水客户端安装目录
//   QISHUI_DEVICE_ID      指定设备号；留空则读/写身份缓存文件
//   QISHUI_IDENTITY_FILE  身份缓存路径，默认 <APPDATA>/HotDownloader/qishui-identity.json
//
// 安全：该服务会看到签名请求的完整请求头（含 Cookie），只在自己信任的机器上跑，
// 并且一定要设置 token；不要把端口暴露到公网。

import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

const PORT = Number(process.env.QISHUI_SIGNER_PORT || 8899);
const HOST = process.env.QISHUI_SIGNER_HOST || '0.0.0.0';
const TOKEN = String(process.env.QISHUI_SIGNER_TOKEN || '').trim();

function identityFile() {
  if (process.env.QISHUI_IDENTITY_FILE) return process.env.QISHUI_IDENTITY_FILE;
  const base = process.env.APPDATA || path.join(os.homedir(), '.config');
  return path.join(base, 'HotDownloader', 'qishui-identity.json');
}

function loadIdentity() {
  const pinned = String(process.env.QISHUI_DEVICE_ID || '').trim();
  if (pinned) {
    return { device_id: pinned, iid: `${pinned}1`, fp: pinned };
  }
  const file = identityFile();
  try {
    const parsed = JSON.parse(fs.readFileSync(file, 'utf8'));
    if (parsed && String(parsed.device_id || '').trim()) {
      return {
        device_id: String(parsed.device_id),
        iid: String(parsed.iid || `${parsed.device_id}1`),
        fp: String(parsed.fp || parsed.device_id),
      };
    }
  } catch {
    // 文件不存在或损坏时重新生成
  }
  const millis = Date.now();
  const identity = { device_id: String(millis), iid: String(millis + 1), fp: String(millis) };
  try {
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, JSON.stringify(identity));
  } catch (error) {
    console.warn('身份缓存写入失败：', error.message);
  }
  return identity;
}

function candidateBdms() {
  const found = [];
  const push = (candidate) => {
    if (candidate && fs.existsSync(candidate)) found.push(candidate);
  };
  push(String(process.env.QISHUI_BDMS_PATH || '').trim());
  const clientDir = String(process.env.QISHUI_CLIENT_DIR || '').trim();
  if (clientDir) {
    push(path.join(clientDir, 'resources', 'app.asar.unpacked', 'bdms.node'));
    push(path.join(clientDir, 'app.asar.unpacked', 'bdms.node'));
  }
  const roots = [];
  if (process.env.LOCALAPPDATA) roots.push(path.join(process.env.LOCALAPPDATA, 'Programs', 'Soda Music'));
  if (process.platform === 'darwin') roots.push('/Applications/SodaMusic.app/Contents');
  if (process.platform === 'linux') roots.push('/opt/SodaMusic');
  for (const root of roots) {
    let entries = [];
    try {
      entries = fs.readdirSync(root, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (!entry.isDirectory()) continue;
      const versionDir = path.join(root, entry.name);
      push(path.join(versionDir, 'resources', 'app.asar.unpacked', 'bdms.node'));
      push(path.join(versionDir, 'app.asar.unpacked', 'bdms.node'));
      // Packages/<版本>/app.asar.unpacked/bdms.node
      try {
        for (const nested of fs.readdirSync(path.join(versionDir), { withFileTypes: true })) {
          if (!nested.isDirectory()) continue;
          push(path.join(versionDir, nested.name, 'app.asar.unpacked', 'bdms.node'));
        }
      } catch {
        // 忽略不可读目录
      }
    }
  }
  return found.sort();
}

const identity = loadIdentity();
const bdmsPath = candidateBdms().pop();
if (!bdmsPath) {
  console.error('未找到 bdms.node：设置 QISHUI_BDMS_PATH 或 QISHUI_CLIENT_DIR');
  process.exit(2);
}

const bdms = require(bdmsPath);
bdms.init({ deviceId: identity.device_id });
console.log(`签名服务已就绪：bdms=${bdmsPath}`);
console.log(`设备号：${identity.device_id}`);
console.log(`监听：http://${HOST}:${PORT}（token ${TOKEN ? '已启用' : '未启用，建议设置 QISHUI_SIGNER_TOKEN'}）`);

function authorized(request) {
  if (!TOKEN) return true;
  const header = String(request.headers.authorization || '');
  return header === `Bearer ${TOKEN}`;
}

function send(response, status, payload) {
  const text = JSON.stringify(payload);
  response.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Content-Length': Buffer.byteLength(text),
  });
  response.end(text);
}

function sign(request) {
  const headers = request.headers || {};
  const headerLines = Object.keys(headers)
    .filter((name) => name && headers[name] !== undefined && headers[name] !== null)
    .map((name) => `${name}\r\n${String(headers[name])}`)
    .join('\r\n');
  const raw = String(bdms.generateHttpSignatureHeaders(String(request.url || ''), headerLines) || '');
  const fields = raw.split('\r\n').filter((item) => item.trim());
  const signed = {};
  for (let index = 0; index + 1 < fields.length; index += 2) {
    signed[fields[index]] = fields[index + 1];
  }
  if (!signed['X-Helios'] || !signed['X-Medusa']) {
    throw new Error(`签名不完整：只拿到 ${Object.keys(signed).join(', ') || '0 个'}`);
  }
  return signed;
}

const server = http.createServer((req, res) => {
  if (!authorized(req)) {
    send(res, 401, { ok: false, error: 'unauthorized' });
    return;
  }
  if (req.method === 'GET' && req.url.startsWith('/config')) {
    send(res, 200, {
      device_id: identity.device_id,
      identity: { device_id: identity.device_id, iid: identity.iid, fp: identity.fp },
    });
    return;
  }
  if (req.method === 'POST' && req.url.startsWith('/sign')) {
    let body = '';
    req.setEncoding('utf8');
    req.on('data', (chunk) => {
      body += chunk;
      if (body.length > 1 << 20) req.destroy();
    });
    req.on('end', () => {
      try {
        const payload = JSON.parse(body || '{}');
        const headers = sign(payload);
        send(res, 200, { ok: true, headers });
      } catch (error) {
        send(res, 200, { ok: false, error: String((error && error.message) || error) });
      }
    });
    return;
  }
  send(res, 404, { ok: false, error: 'not found' });
});

server.listen(PORT, HOST);
