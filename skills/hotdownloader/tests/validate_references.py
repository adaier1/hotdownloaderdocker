#!/usr/bin/env python3
"""校验 references/ 与 test_workflows.md 的结构完整性。

检查项（失败）：
  1. 目录里每个 .md 都有一级标题
  2. 文档之间互相引用的 references/xxx.md 目标必须存在
  3. 不允许占位残留（TODO / 待补充 / 纯文本 / XXX）
  4. 单篇超过 400 行视为臃肿（应拆分或精简）
  5. test_workflows.md 的用例编号按组从 1 连续、不重复

检查项（警告）：
  6. 未被 SKILL.md 或其他文档引用的孤立文档
  7. 没有表格也没有代码块的文档（可执行性差）

用法：
  python .workbuddy/skills/hotdownloader/tests/validate_references.py
退出码：0 通过；1 有失败项。
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

MAX_LINES = 400
PLACEHOLDER_PATTERNS = [
    re.compile(r"TODO"),
    re.compile(r"待补充"),
    re.compile(r"^\s*纯文本\s*$", re.MULTILINE),
    re.compile(r"\bXXX\b"),
    re.compile(r"<待填>"),
]


def check_workflows(path: Path, errors: list[str]) -> int:
    """用例编号必须按组从 1 连续、不重复，否则回归清单会漏项。"""
    if not path.exists():
        errors.append("缺少 tests/test_workflows.md")
        return 0
    text = path.read_text(encoding="utf-8")
    pairs = re.findall(r"^\|\s*([A-Z])(\d+)\s*\|", text, re.MULTILINE)
    if not pairs:
        errors.append("test_workflows.md: 没有识别到用例编号（形如 | A1 |）")
        return 0

    groups: dict[str, list[int]] = {}
    for letter, number in pairs:
        groups.setdefault(letter, []).append(int(number))

    for letter in sorted(groups):
        numbers = groups[letter]
        if len(set(numbers)) != len(numbers):
            duplicated = sorted({n for n in numbers if numbers.count(n) > 1})
            errors.append(f"test_workflows.md: {letter} 组编号重复 {duplicated}")
        expected = list(range(1, len(numbers) + 1))
        if sorted(numbers) != expected:
            errors.append(
                f"test_workflows.md: {letter} 组应为 1..{len(numbers)}，实际 {sorted(numbers)}"
            )
    return len(pairs)


def main() -> int:
    skill_dir = Path(__file__).resolve().parents[1]
    ref_dir = skill_dir / "references"
    errors: list[str] = []
    warnings: list[str] = []

    if not ref_dir.is_dir():
        print(f"[FAIL] 缺少目录 {ref_dir}")
        return 1

    docs = sorted(ref_dir.glob("*.md"))
    if not docs:
        print("[FAIL] references/ 里没有文档")
        return 1

    texts = {doc.name: doc.read_text(encoding="utf-8") for doc in docs}
    skill_text = (skill_dir / "SKILL.md").read_text(encoding="utf-8")

    for name, text in texts.items():
        lines = text.count("\n") + 1
        if not text.startswith("# "):
            errors.append(f"{name}: 第一行必须是一级标题 `# …`")
        if lines > MAX_LINES:
            errors.append(f"{name}: {lines} 行，超过 {MAX_LINES} 行上限")
        for pattern in PLACEHOLDER_PATTERNS:
            if pattern.search(text):
                errors.append(f"{name}: 残留占位内容 `{pattern.pattern}`")
        if "|" not in text and "```" not in text:
            warnings.append(f"{name}: 既无表格也无代码块，可执行性差")

    # 引用目标必须存在
    all_text = {"SKILL.md": skill_text, **texts}
    for name, text in all_text.items():
        for target in set(re.findall(r"references/([\w.-]+\.md)", text)):
            if target not in texts:
                errors.append(f"{name}: 引用了不存在的 references/{target}")

    # 孤立文档
    for name in texts:
        mentioned = any(
            f"references/{name}" in text or name in text
            for other, text in all_text.items()
            if other != name
        )
        if not mentioned:
            warnings.append(f"{name}: 没有被 SKILL.md 或其它文档引用")

    if warnings:
        print(f"[WARN] {len(warnings)} 项提示：")
        for item in warnings:
            print(f"  - {item}")

    cases = check_workflows(skill_dir / "tests/test_workflows.md", errors)

    if errors:
        print(f"[FAIL] {len(errors)} 项错误：")
        for item in errors:
            print(f"  - {item}")
        return 1

    print(f"[OK] {len(docs)} 篇 reference 结构完整，{cases} 条验收用例编号连续")
    return 0


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.exit(main())
