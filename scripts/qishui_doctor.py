#!/usr/bin/env python3
"""汽水音乐登录自检：一次性验证「本机 / MCP / 客户端 / 签名 / 应用状态」。

对应 SKILL.md「开始前：按顺序做这四步」的第 1~3 步，输出逐项结论与下一步建议。
只读检查，不写入任何凭据；不会打印 Cookie 值。

用法：
  python scripts/qishui_doctor.py
  python qishui_doctor.py --dir .workbuddy/hotdownloader
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

sys.path.insert(0, str(Path(__file__).resolve().parent))

import qishui_login as ql  # noqa: E402  同目录复用的实现

OK, BAD, WARN = "✓", "✗", "!"


def display_width(text: str) -> int:
    """中文按 2 列宽计算，供对齐用。"""
    return sum(2 if ord(char) > 0x2E80 else 1 for char in text)


def report(index: str, title: str, mark: str, detail: str, advice: str = "") -> None:
    padding = " " * max(2, 14 - display_width(title))
    print(f"[{index}] {title}{padding}{mark} {detail}")
    if advice:
        print(f"      → {advice}")


def check_platform() -> bool:
    platform = {
        "nt": "Windows",
        "posix": "macOS" if sys.platform == "darwin" else "Linux",
    }.get("nt" if sys.platform.startswith("win") else "posix", sys.platform)
    local = sys.platform in ("win32", "darwin")
    report(
        "1/6",
        "运行环境",
        OK if local else WARN,
        f"{platform} · Python {sys.version.split()[0]}",
        "" if local else "非桌面系统：本机 Cookie 与签名都不存在，只能改走远程签名服务（见 SKILL.md）",
    )
    return local


def check_mcp() -> tuple[bool, str]:
    class _Args:
        mcp_url = None

    try:
        endpoint = ql.mcp_endpoint(_Args())
        result = ql.mcp_call(endpoint, "list_accounts", {})
        text = "".join(
            item.get("text", "")
            for item in result.get("content", [])
            if item.get("type") == "text"
        )
        if '"platforms"' in text or '"accounts"' in text:
            return True, "已连接（list_accounts 可用）"
        return False, f"已连上但返回异常：{text[:80]}"
    except SystemExit as error:
        return False, str(error)
    except Exception as error:  # noqa: BLE001
        return False, f"{type(error).__name__}: {error}"


def check_client() -> tuple[bool, str, str]:
    candidates = ql.client_cookie_paths()
    path = next((item for item in candidates if item.exists()), None)
    if path is None:
        return False, "未找到客户端 Cookie 库", ""
    bdms = ""
    for root in ql.client_install_roots():
        if root.exists():
            found = sorted(root.rglob("bdms.node"))
            if found:
                bdms = str(found[-1])
                break
    return True, str(path), bdms


def check_cookie(path: Path) -> tuple[bool, str]:
    try:
        cookie, names, encrypted = ql.cookies_from_client_db(path)
    except SystemExit as error:
        return False, str(error)
    if not cookie:
        if encrypted:
            return False, f"值被加密（{', '.join(encrypted[:3])}…），需要 DPAPI + AES-GCM 解密"
        return False, "库存在但没有可用 Cookie，客户端里可能没登录"
    missing = [key for key in ql.SESSION_COOKIE_KEYS if key not in names]
    if missing:
        return False, f"缺少会话键 {', '.join(missing)}，请在客户端里重新登录"
    return True, f"{len(names)} 个键 · {len(cookie)} 字符 · 含 sessionid/sid_guard"


def check_signer(bdms: str) -> tuple[bool, str]:
    import os
    import urllib.error
    import urllib.request

    remote = (os.environ.get("QISHUI_SIGNER_URL") or "").strip()
    if remote:
        base = remote.rstrip("/")
        base = base[: -len("/sign")] if base.endswith("/sign") else base
        config_url = f"{base}/config"
        request = urllib.request.Request(config_url, headers={"Accept": "application/json"})
        token = (os.environ.get("QISHUI_SIGNER_TOKEN") or "").strip()
        if token:
            request.add_header("Authorization", f"Bearer {token}")
        try:
            with urllib.request.urlopen(request, timeout=5) as response:
                payload = response.read().decode("utf-8", "replace")
            device = ""
            try:
                import json

                device = str(json.loads(payload).get("device_id", ""))
            except Exception:  # noqa: BLE001
                pass
            return True, f"远程签名服务在线（{remote}，device {device[:6]}…）"
        except (urllib.error.URLError, OSError) as error:
            return False, f"远程签名服务连不上：{remote}（{error}）——它必须一直开着"
    if bdms:
        runtime = ql.find_js_runtime()
        if runtime:
            return True, f"本机 bdms.node + {Path(runtime).name}"
        return False, f"找到 bdms.node（{bdms}）但没有 JS 运行时：装 Node，或用客户端自带 Electron"
    return False, "本机没有 bdms.node（未装客户端）"


def check_app(args) -> tuple[bool, str]:
    class _Args:
        url = args.url
        user = args.user
        password = args.password

    try:
        status = ql.api_call(_Args(), "GET", "/api/qishui/cookie")
    except SystemExit as error:
        return False, str(error)
    if status.get("configured"):
        return True, f"已配置（{status.get('length')} 字符）· 键：{status.get('keys')}"
    return False, "应用里还没有 Cookie"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="汽水音乐登录自检")
    parser.add_argument("--dir", default=str(ql.default_dir()))
    parser.add_argument("--url")
    parser.add_argument("--user")
    parser.add_argument("--password")
    args = parser.parse_args(argv)

    print("汽水音乐登录自检\n")
    check_platform()
    mcp_ok, mcp_detail = check_mcp()
    report(
        "2/6",
        "MCP 连接",
        OK if mcp_ok else BAD,
        mcp_detail,
        "" if mcp_ok else "按 SKILL.md 第 2 步写 ~/.workbuddy/mcp.json 并在连接器管理点「信任」",
    )

    client_ok, client_detail, bdms = check_client()
    report(
        "3/6",
        "汽水客户端",
        OK if client_ok else BAD,
        (client_detail if client_ok else "未安装或未登录过"),
        "" if client_ok else "先安装并登录汽水客户端（https://www.qishui.com/download），否则拿不到 Cookie 也签不了名",
    )

    if client_ok:
        cookie_ok, cookie_detail = check_cookie(Path(client_detail))
        report(
            "4/6",
            "登录 Cookie",
            OK if cookie_ok else BAD,
            cookie_detail,
            "" if cookie_ok else "在客户端里完成登录后重跑本脚本",
        )
    else:
        report("4/6", "登录 Cookie", BAD, "跳过（没有客户端）")

    signer_ok, signer_detail = check_signer(bdms)
    report(
        "5/6",
        "签名能力",
        OK if signer_ok else BAD,
        signer_detail,
        "" if signer_ok else "没有签名：搜索不可用、只有 3 档 AAC、会员曲 60s（详见 SKILL.md 降级矩阵）",
    )

    app_ok, app_detail = check_app(args)
    report(
        "6/6",
        "应用侧状态",
        OK if app_ok else BAD,
        app_detail,
        ""
        if app_ok
        else "python qishui_login.py client --mcp 落库；桌面端用「账号设置 → 汽水音乐」卡片粘贴",
    )

    print()
    if mcp_ok and client_ok and signer_ok and app_ok:
        print("结论：环境齐备，可以登录/搜索/下完整音质。记得签名服务（远程模式）要一直在线。")
        return 0
    if client_ok and not signer_ok:
        print("结论：能登录，但只能解析分享链接 + 3 档 AAC；补上签名后才有搜索与 hi_res/spatial。")
        return 1
    print("结论：环境不完整，先按上面 → 的提示逐项处理。")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
