#!/usr/bin/env python3
"""汽水音乐：在本机一次性解析出 CDN 直链（不需要常驻签名服务）。

用途：把「解析」留在这台装有汽水客户端的机器上，把直链交给服务端下载。
这是汽水下载的**优先路径**：服务端通常没有 `bdms.node`，让它自己签名只会降级。

  python qishui_stream.py 7689447702921349121
  python qishui_stream.py <track_id> --prefer hi_res --filename 320.m4a
  python qishui_stream.py <track_id> --cookie "<cookie>"      # 显式给 Cookie
  python qishui_stream.py <track_id> --cookie-from-client     # 强制读客户端库

输出（stdout，单行 JSON，可直接作为 create_download_task 的 stream 参数）：

  {"url":"https://…","playAuth":"","filename":"320.m4a","size":8414936,"expiresAt":0}

实现：取 Cookie（显式参数 / 环境变量 / 凭据文件 / 本机客户端库）→ 用 node 一次性调
`bdms.node` 拿签名头 → POST track_v2 → 取 url_player_info 里时长完整的变体。
签名进程用完即退，不监听端口、不需要常驻。

环境变量：
  QISHUI_COOKIE          Cookie 字符串（优先于凭据文件）
  QISHUI_COOKIE_FILE    凭据文件，默认 <项目>/data/qishui-credentials.json
  QISHUI_CLIENT_DIR     客户端安装目录
  QISHUI_BDMS_PATH      直接指定 bdms.node
  QISHUI_NODE           指定 node 可执行文件
  QISHUI_DEVICE_ID      指定设备号（默认读身份缓存文件，缺失时按毫秒生成并落盘）
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

API = "https://api.qishui.com/luna/pc/track_v2"
APP_UA = "LunaPC/3.3.0(359450208)"
# 实测：spatial 的地址在无 Cookie 的机器上会被降级，默认优先 hi_res。
PREFERENCE = ["hi_res", "highest", "higher", "medium", "low"]


def project_root() -> Path:
    for parent in Path(__file__).resolve().parents:
        if (parent / "package.json").exists() and (parent / "crates").exists():
            return parent
    return Path.cwd()


def find_bridge_js() -> str:
    """复用仓库里的桥接脚本，避免两份实现漂移。"""
    candidate = (
        project_root()
        / "crates/hotdownloader-core/src/platforms/qishui/assets/qishui-bridge.cjs"
    )
    if candidate.exists():
        return candidate.read_text("utf-8")
    # 兜底：仓库不在时用等价的最小实现。
    return (
        "const fs=require('fs');let raw='';process.stdin.setEncoding('utf8');"
        "process.stdin.on('data',c=>raw+=c);process.stdin.on('end',()=>{try{"
        "const r=JSON.parse(raw||'{}');const p=String(r.bdms||'');"
        "if(!p||!fs.existsSync(p))throw new Error('bdms.node 不存在');"
        "const m=require(p);m.init({deviceId:String(r.device_id||'')});"
        "const h=r.headers||{};const lines=Object.keys(h).map(k=>k+'\\r\\n'+String(h[k])).join('\\r\\n');"
        "const out=String(m.generateHttpSignatureHeaders(String(r.url||''),lines)||'');"
        "const f=out.split('\\r\\n').filter(s=>s.trim());const o={};"
        "for(let i=0;i+1<f.length;i+=2)o[f[i]]=f[i+1];"
        "if(!o['X-Helios']||!o['X-Medusa'])throw new Error('签名头不完整');"
        "process.stdout.write(JSON.stringify({headers:o}));}"
        "catch(e){process.stderr.write(String(e&&e.message||e));process.exit(1);}});"
    )


def find_bdms() -> Path:
    candidates: list[Path] = []
    if env_path := (os.environ.get("QISHUI_BDMS_PATH") or "").strip():
        candidates.append(Path(env_path))
    if client_dir := (os.environ.get("QISHUI_CLIENT_DIR") or "").strip():
        root = Path(client_dir)
        candidates += [
            root / "resources/app.asar.unpacked/bdms.node",
            root / "app.asar.unpacked/bdms.node",
        ]
    roots: list[Path] = []
    if local := os.environ.get("LOCALAPPDATA"):
        roots.append(Path(local) / "Programs" / "Soda Music")
    roots.append(Path("/Applications/SodaMusic.app/Contents"))
    roots.append(Path("/opt/SodaMusic"))
    for root in roots:
        if root.exists():
            candidates += sorted(root.rglob("bdms.node"))
    for candidate in candidates:
        if candidate.exists():
            return candidate
    raise SystemExit("未找到 bdms.node：设置 QISHUI_BDMS_PATH 或 QISHUI_CLIENT_DIR")


def find_node() -> str:
    if pinned := (os.environ.get("QISHUI_NODE") or "").strip():
        if Path(pinned).exists():
            return pinned
    for name in ("node", "node.exe"):
        if found := shutil.which(name):
            return found
    raise SystemExit("未找到 node：装 Node 或设置 QISHUI_NODE")


def identity_file() -> Path:
    return Path(
        os.environ.get("QISHUI_IDENTITY_FILE")
        or Path(os.environ.get("APPDATA") or Path.home() / ".config")
        / "HotDownloader"
        / "qishui-identity.json"
    )


def identity(explicit: str = "") -> dict:
    """设备身份与应用侧同源；缺失时按毫秒生成并落盘，保证本机可独立签名。"""
    device = (explicit or os.environ.get("QISHUI_DEVICE_ID") or "").strip()
    if device:
        return {"device_id": device, "iid": f"{device}1", "fp": device}
    file = identity_file()
    if file.exists():
        try:
            data = json.loads(file.read_text("utf-8"))
        except (OSError, ValueError):
            data = {}
        if str(data.get("device_id", "")).strip():
            return data
    device = str(int(time.time() * 1000))
    payload = {"device_id": device, "iid": f"{device}1", "fp": device}
    try:
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(json.dumps(payload), "utf-8")
        try:
            os.chmod(file, 0o600)
        except OSError:
            pass
        print(f"已生成设备身份：{file}（device {device}）", file=sys.stderr)
    except OSError:
        print(f"未能写入设备身份文件，本次使用临时 device {device}", file=sys.stderr)
    return payload


def _client_cookie() -> str:
    """从本机客户端 Cookie 库读 Cookie，复用 qishui_login.py 的实现（含「退出客户端」提示）。"""
    try:
        import qishui_login as ql
    except ImportError:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        try:
            import qishui_login as ql  # noqa: F811
        except ImportError:
            return ""
    path = next((item for item in ql.client_cookie_paths() if item.exists()), None)
    if path is None:
        return ""
    cookie, names, _encrypted = ql.cookies_from_client_db(path)
    if not cookie:
        return ""
    missing = [key for key in ql.SESSION_COOKIE_KEYS if key not in names]
    if missing:
        print(
            f"警告：客户端 Cookie 缺 {', '.join(missing)}，可能未登录或登录态没落盘；"
            "先完全退出客户端后重跑，或 --cookie 显式指定",
            file=sys.stderr,
        )
    print(f"使用本机客户端 Cookie：{path}", file=sys.stderr)
    return cookie


def cookie(explicit: str = "", from_client: bool = False) -> str:
    """Cookie 来源优先级：--cookie > QISHUI_COOKIE > 凭据文件 > 本机客户端库。"""
    if explicit.strip():
        return explicit.strip()
    env_cookie = (os.environ.get("QISHUI_COOKIE") or "").strip()
    if env_cookie and not from_client:
        return env_cookie
    if not from_client:
        file = Path(
            os.environ.get("QISHUI_COOKIE_FILE")
            or project_root() / "data" / "qishui-credentials.json"
        )
        if file.exists():
            try:
                raw = file.read_text("utf-8")
            except OSError as error:
                raise SystemExit(f"读不到凭据文件：{file}（{error}）") from None
            try:
                parsed = json.loads(raw)
            except ValueError:
                parsed = {}
            value = str((parsed or {}).get("cookie", "")).strip() if isinstance(parsed, dict) else ""
            if not value and raw.strip() and "=" in raw and "\n" not in raw.strip():
                value = raw.strip()  # 允许直接放一行 Cookie
            if value:
                return value
    client = _client_cookie()
    if client:
        return client
    raise SystemExit(
        "没有可用 Cookie：客户端未安装/未登录，凭据文件也不存在。"
        "先 python scripts/qishui_login.py client 落库，或 --cookie '<cookie>' / 设置 QISHUI_COOKIE"
    )


def sign(url: str, body: str, headers: dict, bdms: Path, node: str, device_id: str) -> dict:
    payload = json.dumps(
        {
            "bdms": str(bdms),
            "device_id": device_id,
            "url": url,
            "method": "POST",
            "body": body,
            "headers": headers,
        }
    )
    result = subprocess.run(
        [node, "-e", find_bridge_js()],
        input=payload,
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise SystemExit(f"签名失败：{result.stderr.strip() or '未知原因'}")
    return json.loads(result.stdout)["headers"]


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="本机解析汽水直链（一次性，无需常驻服务）")
    parser.add_argument("track_id")
    parser.add_argument("--prefer", default="hi_res", help="优先档位，默认 hi_res")
    parser.add_argument("--filename", help="输出文件名，默认按档位推断")
    parser.add_argument("--cookie", default="", help="显式 Cookie（默认按凭据文件/客户端库自动找）")
    parser.add_argument(
        "--cookie-from-client",
        action="store_true",
        help="强制从本机汽水客户端 Cookie 库读取（忽略凭据文件与环境变量）",
    )
    parser.add_argument("--device-id", default="", help="指定设备号，默认读身份文件")
    args = parser.parse_args(argv)

    bdms, node = find_bdms(), find_node()
    ident = identity(args.device_id)
    cookie_value = cookie(args.cookie, args.cookie_from_client)
    device_id = str(ident.get("device_id"))

    params = {
        "aid": "386088", "app_name": "luna_pc", "region": "cn", "geo_region": "cn",
        "os_region": "cn", "sim_region": "", "device_id": device_id, "cdid": "",
        "iid": str(ident.get("iid") or ""), "version_name": "3.3.0",
        "version_code": "30030000", "channel": "official", "build_mode": "master",
        "network_carrier": "", "ac": "wifi", "tz_name": "Asia/Shanghai", "resolution": "",
        "device_platform": "windows", "device_type": "Windows", "os_version": "Windows 11",
        "fp": str(ident.get("fp") or device_id),
    }
    url = API + "?" + urllib.parse.urlencode(params)
    body = json.dumps(
        {
            "track_id": args.track_id,
            "media_type": "track",
            "queue_type": "favorite_track_playlist",
            "scene_name": "library",
        }
    )

    import hashlib

    headers = {
        "User-Agent": APP_UA,
        "Content-Type": "application/json; charset=utf-8",
        "X-SS-STUB": hashlib.md5(body.encode()).hexdigest().upper(),
        "x-luna-background-type": "foreground",
        "x-luna-is-background-req": "0",
        "x-luna-is-local-user": "1",
        "Cookie": cookie_value,
    }
    signed = sign(url, body, headers, bdms, node, device_id)

    request = urllib.request.Request(
        url, data=body.encode(), method="POST", headers={**headers, **signed}
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            track = json.loads(response.read().decode("utf-8", "replace"))
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", "replace")[:200]
        raise SystemExit(
            f"请求 track_v2 失败 HTTP {error.code}：{detail}；"
            "Cookie 可能已过期——试 --cookie-from-client，或先 python scripts/qishui_login.py client 重新落库"
        ) from None
    except urllib.error.URLError as error:
        raise SystemExit(f"连不上汽水接口：{error.reason}")
    info_url = (track.get("track_player") or {}).get("url_player_info", "")
    if not info_url:
        raise SystemExit(
            "响应里没有 url_player_info（签名或设备身份可能不被接受）；"
            "换 --cookie-from-client 重试，或删除身份文件后重跑"
        )

    with urllib.request.urlopen(
        urllib.request.Request(info_url, headers={"User-Agent": "Mozilla/5.0"}), timeout=30
    ) as response:
        plays = ((json.loads(response.read()) or {}).get("Result") or {}).get("Data", {}).get(
            "PlayInfoList"
        ) or []
    full = [item for item in plays if float(item.get("Duration") or 0) > 180]
    pool = full or plays
    if not pool:
        raise SystemExit("没有可用变体")

    prefer = args.prefer.lower()
    ordered = sorted(
        pool,
        key=lambda item: (
            0
            if str(item.get("Quality", "")).lower() == prefer
            else 1 + (PREFERENCE.index(str(item.get("Quality", "")).lower())
                      if str(item.get("Quality", "")).lower() in PREFERENCE else 99)
        ),
    )
    best = ordered[0]
    quality = str(best.get("Quality", "")).lower()
    filename = args.filename or ("flac.flac" if quality == "lossless" else "320.m4a")
    expire = 0
    for part in str(best.get("MainPlayUrl", "")).split("&"):
        if part.startswith("expire="):
            expire = int(part.split("=", 1)[1]) * 1000
    stream = {
        "url": best.get("MainPlayUrl", ""),
        "playAuth": best.get("PlayAuth", "") or "",
        "filename": filename,
        "size": int(best.get("Size") or 0),
        "expiresAt": expire,
    }
    print(
        f"档位 {quality or '未知'} · {stream['size']} 字节 · {filename}"
        + ("" if quality == prefer else f"（首选 {prefer} 不可用，已降级）"),
        file=sys.stderr,
    )
    print(json.dumps(stream, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
