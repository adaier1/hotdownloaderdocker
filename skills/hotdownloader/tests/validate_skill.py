#!/usr/bin/env python3
"""校验 hotdownloader skill 的文档与 Rust 源码是否一致。

检查项：
  1. SKILL.md frontmatter 必填字段
  2. 文档里出现的 MCP 工具名是否真实存在（对照 mcp.rs 的分发分支）
  3. 文档里出现的 HTTP 路径是否存在（对照 http.rs 的静态路由与动态前缀）
  4. SKILL.md 引用的 references/*.md 是否都存在
  5. SKILL.md 提到的 scripts/* 是否都存在

用法：
  python .workbuddy/skills/hotdownloader/tests/validate_skill.py
  python .../validate_skill.py --skill-dir <目录> --repo-root <仓库根>

退出码：0 全部通过；1 有失败项。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# 文档里以这些前缀出现的 snake_case 词被当作"工具名"来校验。
TOOL_PREFIXES = (
    "search_", "fetch_", "parse_", "create_", "list_", "get_", "patch_",
    "save_", "start_", "check_", "pause_", "resume_", "retry_", "remove_",
    "rename_", "write_", "delete_", "login_", "logout_",
)

# 这些是模板/占位，不算工具名。
IGNORE_SNAKE = {
    "task_id", "task_ids", "song_id", "song_mid", "media_mid", "play_auth",
    "expires_at", "file_path", "error_msg", "retry_count", "quality_order",
    "auto_downgrade", "download_dir", "naming_template", "duplicate_action",
    "desired_quality", "delete_file", "has_more", "file_size", "added_at",
    "cover_url", "available_qualities", "new_name", "song_title",
    "run_in_background", "as_str",
    # 接口查询参数，不是工具
    "search_id", "search_scene", "search_method", "from_search_id",
}


def fail(errors: list[str], message: str) -> None:
    errors.append(message)


def repo_root_from(skill_dir: Path) -> Path | None:
    for parent in [skill_dir, *skill_dir.parents]:
        if (parent / "crates").is_dir() and (parent / "package.json").exists():
            return parent
    return None


def parse_frontmatter(text: str) -> dict[str, str]:
    if not text.startswith("---"):
        return {}
    end = text.find("\n---", 3)
    if end < 0:
        return {}
    block = text[3:end]
    data: dict[str, str] = {}
    for line in block.splitlines():
        if not line.strip() or line.startswith((" ", "\t")):
            continue
        if ":" not in line:
            continue
        key, _, value = line.partition(":")
        data[key.strip()] = value.strip()
    return data


def real_tool_names(mcp_rs: Path) -> set[str]:
    """只取 call_tool 里按工具名的顶层分派，避免把内部 match 分支当成工具。"""
    source = mcp_rs.read_text(encoding="utf-8")
    marker = "match name {"
    block = source.split(marker, 1)[1] if marker in source else source
    block = block.split("\n    };", 1)[0]
    return set(re.findall(r'^\s*"([a-z][a-z0-9_]+)"\s*=>', block, re.MULTILINE))


def real_http_routes(http_rs: Path) -> tuple[set[str], list[str]]:
    source = http_rs.read_text(encoding="utf-8")
    static = set(re.findall(r'Method::\w+\s*,\s*"([^"]+)"\s*\)', source))
    static |= set(re.findall(r'path\s*==\s*"([^"]+)"', source))
    dynamic = re.findall(r'path\.starts_with\(\s*"([^"]+)"\s*\)', source)
    # "/api/" 只是"是否走进 API 分支"的粗判断，不能当作"任意 /api/xxx 都合法"。
    dynamic = [prefix for prefix in dynamic if prefix.rstrip("/") != "/api"]
    return static, dynamic


def documented_paths(text: str) -> set[str]:
    found = set()
    for raw in re.findall(r"(?<![\w>])(/(?:api|healthz|mcp)(?:/[\w{}-]+)*)", text):
        path = raw.rstrip(".,;:)'\"`")
        if path in {"/api", "/api/"}:
            continue
        found.add(path)
    return found


def rust_function_names(repo_root: Path) -> set[str]:
    """文档里引用内部函数（如 retry_task_with_quality）不算虚构工具。"""
    names: set[str] = set()
    for path in repo_root.glob("crates/**/*.rs"):
        try:
            names |= set(re.findall(r"\bfn\s+([a-z_][a-z0-9_]*)", path.read_text(encoding="utf-8")))
        except OSError:
            continue
    return names


def tool_words(text: str) -> set[str]:
    words = set(re.findall(r"`([a-z][a-z0-9_]{3,})`", text))
    return {
        word for word in words
        if word.startswith(TOOL_PREFIXES) and word not in IGNORE_SNAKE and "_" in word[1:]
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skill-dir", default=str(Path(__file__).resolve().parents[1]))
    parser.add_argument("--repo-root", default=None)
    args = parser.parse_args()

    skill_dir = Path(args.skill_dir).resolve()
    repo_root = Path(args.repo_root).resolve() if args.repo_root else repo_root_from(skill_dir)
    errors: list[str] = []

    skill_md = skill_dir / "SKILL.md"
    if not skill_md.exists():
        print(f"[FAIL] 缺少 {skill_md}")
        return 1

    skill_text = skill_md.read_text(encoding="utf-8")
    meta = parse_frontmatter(skill_text)
    for key in ("name", "description", "version"):
        if not meta.get(key):
            fail(errors, f"SKILL.md frontmatter 缺少 {key}")
    if meta.get("name") and meta["name"] != "hotdownloader":
        fail(errors, f"frontmatter name 应为 hotdownloader，实际 {meta['name']}")
    if meta.get("description") and len(meta["description"]) < 40:
        fail(errors, "description 过短，无法让 agent 判断何时使用")

    if repo_root is None:
        print("[SKIP] 未找到仓库根（需同时含 crates/ 与 package.json），跳过源码交叉校验")
        return 0 if not errors else 1

    mcp_rs = repo_root / "crates/hotdownloader-server/src/mcp.rs"
    http_rs = repo_root / "crates/hotdownloader-server/src/http.rs"
    if not mcp_rs.exists() or not http_rs.exists():
        fail(errors, "找不到 mcp.rs / http.rs，无法做源码交叉校验")
        report(errors)
        return 1

    tools = real_tool_names(mcp_rs)
    static_routes, dynamic_prefixes = real_http_routes(http_rs)
    rust_fns = rust_function_names(repo_root)

    docs = [skill_md, *sorted((skill_dir / "references").glob("*.md"))]
    for doc in docs:
        text = doc.read_text(encoding="utf-8")
        rel = doc.relative_to(repo_root) if repo_root in doc.parents else doc.name

        for word in sorted(tool_words(text)):
            if word in tools or word in rust_fns:
                continue
            fail(errors, f"{rel}: 提到 `{word}`，但 mcp.rs 里没有这个工具")

        for path in sorted(documented_paths(text)):
            if path in static_routes:
                continue
            if any(path.startswith(prefix) for prefix in dynamic_prefixes):
                continue
            fail(errors, f"{rel}: 提到 {path}，但 http.rs 里没有这条路由")

    # SKILL.md 必须引用全部 references
    ref_dir = skill_dir / "references"
    for ref in sorted(ref_dir.glob("*.md")):
        if ref.name not in skill_text:
            fail(errors, f"SKILL.md 未索引 references/{ref.name}")

    # 文档里提到的 scripts 必须存在
    for name in sorted(set(re.findall(r"scripts/([\w.-]+\.(?:py|mjs|js))", skill_text))):
        if not (repo_root / "scripts" / name).exists():
            fail(errors, f"SKILL.md 提到 scripts/{name}，但仓库里没有这个文件")

    report(errors)
    return 0 if not errors else 1


def report(errors: list[str]) -> None:
    if not errors:
        print("[OK] skill 文档与源码一致")
        return
    print(f"[FAIL] {len(errors)} 项不一致：")
    for item in errors:
        print(f"  - {item}")


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.exit(main())
