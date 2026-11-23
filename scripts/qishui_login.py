#!/usr/bin/env python3
"""汽水音乐登录：从本机客户端读取 Cookie，并写入运行中的 HotDownloader。

汽水没有网页版，也不支持账号密码登录；唯一可行的取凭据方式是读桌面客户端的
Chromium Cookie 库。签名（`x-helios`/`x-medusa`）由客户端里的 `bdms.node` 生成，
不属于本脚本职责——本脚本只负责「拿到 Cookie 并落库」，环境自检见 `qishui_doctor.py`。

用法：
  python qishui_login.py client                 # 只读取，打印键名与摘要，写临时副本
  python qishui_login.py client --write         # 读取并通过 REST 写入应用
  python qishui_login.py client --mcp           # 读取并通过 MCP save_cookie 写入
  python qishui_login.py save --cookie-file c.txt
  python qishui_login.py status                 # 查询应用当前 Cookie 状态

安全约定：Cookie 不打印值；临时副本写 0600，落库成功后请删除。
运行中提示：汽水客户端开着时 Cookie 库可能被占、登录态也可能没落盘；读不到值（空库 /
缺 sessionid / 库被锁）时脚本会提示先完全退出客户端（含托盘）再重读，不会自动关进程。
"""

from __future__ import annotations

import argparse
import base64
import csv
import io
import json
import os
import sqlite3
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from pathlib import Path

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

SESSION_COOKIE_KEYS = ("sessionid", "sessionid_ss", "sid_tt", "sid_guard")
MAX_COOKIE_LEN = 16_384


# ---------------------------------------------------------------- 通用


def project_root() -> Path | None:
    """向上找到含 `package.json` 与 `crates/` 的目录，用于定位 `.env` 与 `data/`。"""
    for parent in Path(__file__).resolve().parents:
        if (parent / "package.json").exists() and (parent / "crates").exists():
            return parent
    return None


def default_dir() -> Path:
    """临时文件目录，默认在项目 `.workbuddy/hotdownloader`。"""
    env = os.environ.get("QISHUI_LOGIN_DIR", "").strip()
    if env:
        return Path(env)
    root = project_root()
    base = root if root else Path(os.environ.get("TEMP") or "/tmp")
    return base / ".workbuddy" / "hotdownloader"


def mask(value: str, keep: int = 6) -> str:
    value = (value or "").strip()
    if not value:
        return "(空)"
    if len(value) <= keep:
        return "***"
    return f"{value[:keep]}...（共 {len(value)} 字符）"


def read_dotenv() -> dict[str, str]:
    root = project_root()
    if not root:
        return {}
    path = root / ".env"
    if not path.exists():
        return {}
    values: dict[str, str] = {}
    for line in path.read_text("utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip().strip('"').strip("'")
    return values


# ---------------------------------------------------------------- 应用侧写入


def api_base(args) -> str:
    if getattr(args, "url", None):
        return args.url.rstrip("/")
    env = read_dotenv()
    return f"http://127.0.0.1:{env.get('WEB_PORT') or '8791'}"


def api_headers(args) -> dict[str, str]:
    env = read_dotenv()
    user = getattr(args, "user", None) or env.get("AUTH_USERNAME") or "admin"
    password = getattr(args, "password", None) or env.get("AUTH_PASSWORD") or ""
    token = base64.b64encode(f"{user}:{password}".encode()).decode()
    return {"Authorization": f"Basic {token}", "Content-Type": "application/json"}


def api_call(args, method: str, path: str, payload: dict | None = None) -> dict:
    url = api_base(args) + path
    data = json.dumps(payload, ensure_ascii=False).encode() if payload is not None else None
    request = urllib.request.Request(url, data=data, headers=api_headers(args), method=method)
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            return json.loads(response.read().decode("utf-8", "replace") or "{}")
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", "replace")
        raise SystemExit(f"{method} {path} 失败 HTTP {error.code}: {body[:300]}")
    except urllib.error.URLError as error:
        raise SystemExit(f"连不上 {url}：{error.reason}（应用/容器是否在运行？）")


def mcp_endpoint(args) -> str:
    if getattr(args, "mcp_url", None):
        return args.mcp_url
    env = read_dotenv()
    port = env.get("WEB_PORT") or "8791"
    key = ""
    root = project_root()
    if root:
        path = root / "data" / "mcp.json"
        if path.exists():
            try:
                key = (json.loads(path.read_text("utf-8")).get("key") or "").strip()
            except (OSError, ValueError):
                key = ""
    if not key:
        raise SystemExit("读不到 data/mcp.json 的 key，请用 --mcp-url 给完整地址")
    return f"http://127.0.0.1:{port}/mcp?key={key}"


def _mcp_post(url: str, session: str, payload: dict) -> tuple[dict, str]:
    headers = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream"}
    if session:
        headers["Mcp-Session-Id"] = session
    request = urllib.request.Request(
        url, data=json.dumps(payload).encode(), headers=headers, method="POST"
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            sid = response.headers.get("Mcp-Session-Id") or session
            raw = response.read().decode("utf-8", "replace")
            content_type = response.headers.get("Content-Type", "")
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", "replace")
        raise SystemExit(f"MCP HTTP {error.code}: {body[:300]}")
    except urllib.error.URLError as error:
        raise SystemExit(f"连不上 MCP：{error.reason}")
    if not raw.strip():
        return {}, sid
    if "text/event-stream" in content_type:
        for line in raw.splitlines():
            if line.startswith("data:"):
                chunk = line[5:].strip()
                if chunk:
                    return json.loads(chunk), sid
        return {}, sid
    return json.loads(raw), sid


def mcp_call(url: str, tool: str, arguments: dict) -> dict:
    session = ""
    _, session = _mcp_post(url, session, {
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "hotdownloader-skill", "version": "2.0.0"},
        },
    })
    _mcp_post(url, session, {"jsonrpc": "2.0", "method": "notifications/initialized"})
    response, _ = _mcp_post(url, session, {
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {"name": tool, "arguments": arguments},
    })
    if response.get("error"):
        raise SystemExit(f"MCP 调用失败：{response['error']}")
    return response.get("result") or {}


def persist_cookie(args, cookie: str, source_label: str) -> int:
    if not cookie or "=" not in cookie:
        raise SystemExit("Cookie 为空或格式不正确")
    if len(cookie) > MAX_COOKIE_LEN:
        raise SystemExit("Cookie 过长，请确认内容是否完整")
    if getattr(args, "mcp", False) or getattr(args, "mcp_url", None):
        endpoint = mcp_endpoint(args)
        result = mcp_call(endpoint, "save_cookie", {"platform": "qishui", "cookie": cookie})
        print(f"已通过 MCP save_cookie 写入（来源：{source_label}）：")
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        status = api_call(args, "POST", "/api/qishui/cookie", {"cookie": cookie})
        print(f"已写入应用（来源：{source_label}）：")
        print(json.dumps(status, ensure_ascii=False, indent=2))
    return 0


# ---------------------------------------------------------------- 客户端 Cookie


def client_cookie_paths() -> list[Path]:
    """Chromium Cookie 库位置：Windows / macOS / Linux 各一套。"""
    candidates: list[Path] = []
    appdata = os.environ.get("APPDATA")
    if appdata:
        candidates.append(Path(appdata) / "SodaMusic" / "Network" / "Cookies")
    home = Path.home()
    candidates.append(home / "Library" / "Application Support" / "SodaMusic" / "Network" / "Cookies")
    candidates.append(home / ".config" / "SodaMusic" / "Network" / "Cookies")
    return candidates


def client_install_roots() -> list[Path]:
    """客户端安装根目录；用于定位 `bdms.node` 与 Electron 可执行文件。"""
    roots: list[Path] = []
    env_dir = (os.environ.get("QISHUI_CLIENT_DIR") or "").strip()
    if env_dir:
        roots.append(Path(env_dir))
    local = os.environ.get("LOCALAPPDATA")
    if local:
        roots.append(Path(local) / "Programs" / "Soda Music")
    home = Path.home()
    roots.append(home / "Applications" / "SodaMusic.app" / "Contents")
    roots.append(Path("/Applications/SodaMusic.app/Contents"))
    roots.append(Path("/opt/SodaMusic"))
    return roots


CLIENT_PROCESS_MARKERS = ("sodamusic", "汽水音乐")
CLOSE_CLIENT_STEPS = (
    "请完全退出客户端：托盘图标右键 → 退出（任务管理器里确认没有 SodaMusic 进程），"
    "再重跑本条命令；读完 Cookie 之后再打开客户端不影响使用。"
)


def client_processes() -> list[tuple[str, str]]:
    """探测正在运行的汽水客户端进程，返回 [(pid, 名称)]；只读探测，失败就当没运行。"""
    markers = tuple(marker.replace(" ", "") for marker in CLIENT_PROCESS_MARKERS)

    def hit(name: str) -> bool:
        normalized = name.lower().replace(" ", "")
        return any(marker in normalized for marker in markers)

    found: list[tuple[str, str]] = []
    try:
        if sys.platform.startswith("win"):
            completed = subprocess.run(
                ["tasklist", "/FO", "CSV", "/NH"],
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=10,
                creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            )
            for row in csv.reader(io.StringIO(completed.stdout or "")):
                if len(row) >= 2 and row[1].strip().isdigit() and hit(row[0]):
                    found.append((row[1].strip(), row[0].strip()))
        else:
            completed = subprocess.run(
                ["ps", "-Ao", "pid=,command="],
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=10,
            )
            for line in (completed.stdout or "").splitlines():
                parts = line.strip().split(None, 1)
                if len(parts) == 2 and parts[0].isdigit() and hit(parts[1]):
                    found.append((parts[0], Path(parts[1].split()[0]).name))
    except (OSError, subprocess.SubprocessError, ValueError):
        return []
    return found


def client_running_note() -> str:
    """客户端正在运行时返回一段可执行提示；没在运行返回空串。"""
    procs = client_processes()
    if not procs:
        return ""
    names = "、".join(f"{name}(PID {pid})" for pid, name in procs[:3])
    if len(procs) > 3:
        names += f" 等 {len(procs)} 个进程"
    return (
        f"检测到汽水音乐客户端正在运行：{names}。"
        "客户端运行时 Cookie 库被占用、最新登录态也可能还没写盘，读到空值或缺 sessionid 多半是这个原因。"
        + CLOSE_CLIENT_STEPS
    )


def close_client_advice() -> str:
    """一行版提示，供自检脚本与文档复用。"""
    return (
        "先完全退出汽水音乐客户端（托盘右键退出，任务管理器确认没有 SodaMusic 进程）再重跑；"
        "客户端运行时 Cookie 库被占用、登录态未落盘，常读不到值或缺 sessionid"
    )


def with_client_hint(message: str) -> str:
    """给报错补上「客户端在运行」的提示；不在运行就原样返回。"""
    note = client_running_note()
    return f"{message}\n{note}" if note else message


def find_js_runtime() -> str:
    """签名可用的 JS 运行时：QISHUI_NODE > PATH 里的 node > 客户端自带 Electron。"""
    pinned = (os.environ.get("QISHUI_NODE") or "").strip()
    if pinned and Path(pinned).exists():
        return pinned
    for directory in (os.environ.get("PATH") or "").split(os.pathsep):
        for name in ("node.exe", "node"):
            candidate = Path(directory) / name
            if candidate.is_file():
                return str(candidate)
    for root in client_install_roots():
        for name in ("SodaMusic.exe", "SodaMusic", "soda-music"):
            for candidate in root.rglob(name):
                if candidate.is_file():
                    return str(candidate)
    return ""


def read_locked_bytes(path: Path) -> bytes:
    """客户端运行时 Cookies 被独占锁；Windows 下用共享标志只读打开，不改动文件。"""
    if os.name != "nt":
        return path.read_bytes()
    import ctypes
    import ctypes.wintypes as wintypes

    kernel32 = ctypes.windll.kernel32
    handle = kernel32.CreateFileW(str(path), 0x80000000, 0x1 | 0x2 | 0x4, None, 3, 0x80, None)
    if not handle or handle == ctypes.c_void_p(-1).value:
        raise SystemExit(f"打不开客户端 Cookie 库：{path}（权限或占用问题）")
    try:
        size = kernel32.GetFileSize(ctypes.c_void_p(handle), None)
        buffer = ctypes.create_string_buffer(size)
        read = wintypes.DWORD(0)
        kernel32.ReadFile(ctypes.c_void_p(handle), buffer, size, ctypes.byref(read), None)
        return buffer.raw[: read.value]
    finally:
        kernel32.CloseHandle(ctypes.c_void_p(handle))


def cookies_from_client_db(path: Path) -> tuple[str, list[str], list[str]]:
    """返回 (cookie 串, 用到的键名, 仍被加密的键名)。

    客户端运行时库可能被占、或最新登录态还压在 WAL 里没落盘；这类失败都要带上
    「先完全退出客户端」的提示，不吞原始原因。
    """
    try:
        raw = read_locked_bytes(path)
    except SystemExit as error:
        raise SystemExit(with_client_hint(str(error))) from None
    except OSError as error:
        raise SystemExit(with_client_hint(f"读取客户端 Cookie 库失败：{path}（{error}）")) from None
    if not raw:
        raise SystemExit(with_client_hint(f"客户端 Cookie 库是空的：{path}（客户端可能正在写入）"))

    temp_path = ""
    connection = sqlite3.connect(":memory:")
    try:
        try:
            connection.deserialize(raw)
        except (AttributeError, sqlite3.DatabaseError):
            # 老 Python 没有 deserialize，或主库不含 WAL 里的新数据：落只读副本再试
            with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as handle:
                handle.write(raw)
                temp_path = handle.name
            connection.close()
            connection = sqlite3.connect(temp_path)
        rows = connection.execute(
            "select host_key, name, value, length(encrypted_value) from cookies"
            " where host_key like '%qishui%' order by name"
        ).fetchall()
    except sqlite3.DatabaseError as error:
        raise SystemExit(
            with_client_hint(f"解析客户端 Cookie 库失败：{path}（{error}）；库可能正在被客户端写入")
        ) from None
    finally:
        connection.close()
        if temp_path:
            try:
                os.unlink(temp_path)
            except OSError:
                pass
    plain: list[tuple[str, str]] = []
    encrypted: list[str] = []
    for _host, name, value, encrypted_len in rows:
        if value:
            plain.append((name, value))
        elif encrypted_len:
            encrypted.append(name)
    cookie = "; ".join(f"{name}={value}" for name, value in plain)
    return cookie, [name for name, _ in plain], encrypted


# ---------------------------------------------------------------- 子命令


def cmd_client(args) -> int:
    paths = [Path(item) for item in args.cookie_db] if args.cookie_db else client_cookie_paths()
    path = next((item for item in paths if item.exists()), None)
    if path is None:
        raise SystemExit(
            "未找到汽水客户端 Cookie 库：本机可能没有安装客户端，或从未登录过；"
            "也可用 --cookie-db 指定路径"
        )
    procs = client_processes()
    if procs:
        print(
            f"注意：汽水音乐客户端正在运行（{len(procs)} 个进程）。"
            "若下面读到空值/缺 sessionid，按提示完全退出客户端再试。"
        )
    cookie, names, encrypted = cookies_from_client_db(path)
    print(f"已读取客户端 Cookie 库：{path}")
    print(f"  键名（{len(names)} 个）: {', '.join(names) if names else '无'}")
    if encrypted and not cookie:
        raise SystemExit(
            with_client_hint(
                "该库的 Cookie 值是加密的（Chromium v10/v11），需要 DPAPI + AES-GCM 解密；"
                "这条路径暂未实现，请改用其它方式获取 Cookie"
            )
        )
    if not cookie:
        raise SystemExit(
            with_client_hint(
                f"客户端 Cookie 库里没有可用 Cookie：{path}。"
                "要么客户端里还没登录，要么登录态还没写盘"
            )
        )
    missing = [key for key in SESSION_COOKIE_KEYS if key not in names]
    if missing:
        print(f"  注意：缺少会话键 {', '.join(missing)}，客户端里可能没有登录")
        if procs:
            print("        客户端正在运行，登录态可能还没落盘：完全退出客户端后重跑一次再判断。")
    print(f"  Cookie 摘要: {mask(cookie)}")

    state_dir = Path(args.dir)
    state_dir.mkdir(parents=True, exist_ok=True)
    out = state_dir / "cookie-client.txt"
    out.write_text(cookie, "utf-8")
    try:
        os.chmod(out, 0o600)
    except OSError:
        pass
    print(f"  已存临时副本: {out}（用完请删除）")

    if args.write or args.mcp or args.mcp_url:
        return persist_cookie(args, cookie, "本机客户端")
    print("  （未写入应用；加 --write 或 --mcp 即可落库）")
    return 0


def cmd_save(args) -> int:
    if args.cookie_file:
        cookie = Path(args.cookie_file).read_text("utf-8").strip()
    elif args.cookie:
        cookie = args.cookie.strip()
    else:
        raise SystemExit("需要 --cookie-file 或 --cookie")
    return persist_cookie(args, cookie, "手工输入")


def cmd_status(args) -> int:
    status = api_call(args, "GET", "/api/qishui/cookie")
    print(json.dumps(status, ensure_ascii=False, indent=2))
    return 0 if status.get("configured") else 5


# ---------------------------------------------------------------- 入口


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="汽水音乐：从本机客户端读 Cookie 并落库")
    parser.add_argument("--dir", default=str(default_dir()), help="临时文件目录（默认项目 .workbuddy/hotdownloader）")
    sub = parser.add_subparsers(dest="command", required=True)

    client = sub.add_parser("client", help="从本机汽水客户端读取 Cookie")
    client.add_argument("--cookie-db", action="append", help="Cookie 库路径，可重复；默认自动探测")
    client.add_argument("--write", action="store_true", help="读出来后写入应用（REST）")
    client.add_argument("--mcp", action="store_true", help="读出来后通过 MCP save_cookie 写入")
    client.add_argument("--mcp-url", help="完整 MCP 地址")
    client.add_argument("--url", help="应用地址，默认取 .env 的 WEB_PORT")
    client.add_argument("--user")
    client.add_argument("--password")

    save = sub.add_parser("save", help="写入已有 Cookie 字符串/文件")
    save.add_argument("--cookie-file", help="Cookie 文本文件")
    save.add_argument("--cookie", help="Cookie 字符串（注意 shell 历史记录）")
    save.add_argument("--url", help="应用地址，默认取 .env 的 WEB_PORT")
    save.add_argument("--mcp", action="store_true", help="改用 MCP save_cookie 写入")
    save.add_argument("--mcp-url", help="完整 MCP 地址")
    save.add_argument("--user")
    save.add_argument("--password")

    status = sub.add_parser("status", help="查询应用 Cookie 状态")
    status.add_argument("--url")
    status.add_argument("--user")
    status.add_argument("--password")

    args = parser.parse_args(argv)
    handler = {"client": cmd_client, "save": cmd_save, "status": cmd_status}[args.command]
    return handler(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
