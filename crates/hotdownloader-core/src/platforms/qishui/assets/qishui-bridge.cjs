// 汽水签名垫片：从 stdin 读一条请求，用官方客户端的 bdms.node 算出签名头，写回 stdout。
//
// 输入：{"bdms":"<bdms.node 路径>","device_id":"<设备号>","url":"<完整 URL>",
//        "headers":{"User-Agent":"...","Cookie":"..."}}
// 输出：{"headers":{"X-Helios":"...","X-Medusa":"..."}}，失败时退出码非 0、原因写 stderr。
//
// bdms.node 由汽水客户端附带，只暴露两个能力：init({deviceId}) 与
// generateHttpSignatureHeaders(url, headerLines)，后者返回 "名称\r\n值\r\n名称\r\n值" 的成对文本。

const fs = require('fs');

let raw = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk) => {
  raw += chunk;
});

process.stdin.on('end', () => {
  try {
    const request = JSON.parse(raw || '{}');
    const bdmsPath = String(request.bdms || '').trim();
    const deviceId = String(request.device_id || '').trim();
    if (!bdmsPath || !fs.existsSync(bdmsPath)) {
      throw new Error('bdms.node 不存在: ' + bdmsPath);
    }
    if (!deviceId) {
      throw new Error('缺少 device_id');
    }

    const bdms = require(bdmsPath);
    bdms.init({ deviceId });

    const headers = request.headers || {};
    const headerLines = Object.keys(headers)
      .filter((name) => name && headers[name] !== undefined && headers[name] !== null)
      .map((name) => name + '\r\n' + String(headers[name]))
      .join('\r\n');

    const signed = String(bdms.generateHttpSignatureHeaders(String(request.url || ''), headerLines) || '');
    const fields = signed.split('\r\n').filter((item) => item.trim().length > 0);
    const result = {};
    for (let index = 0; index + 1 < fields.length; index += 2) {
      result[fields[index]] = fields[index + 1];
    }
    if (!result['X-Helios'] || !result['X-Medusa']) {
      throw new Error('签名头不完整，只拿到: ' + Object.keys(result).join(', '));
    }
    process.stdout.write(JSON.stringify({ headers: result }));
  } catch (error) {
    process.stderr.write(String((error && error.message) || error));
    process.exit(1);
  }
});
