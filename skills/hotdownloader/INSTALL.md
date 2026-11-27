# 安装 hotdownloader Skill

这份文件写给 **agent**：用户把下面这句话发给你，你就照这里执行。

> 请按这个说明安装 hotdownloader Skill：`https://raw.githubusercontent.com/adaier1/hotdownloaderdocker/main/skills/hotdownloader/INSTALL.md`

## 目标

把 `skills/hotdownloader/` 装到 agent 能读到的 skills 目录，二选一：

| 级别 | 路径 | 适用 |
| --- | --- | --- |
| 用户级（推荐） | `~/.workbuddy/skills/hotdownloader`（Windows：`%USERPROFILE%\.workbuddy\skills\hotdownloader`） | 所有项目都能用 |
| 项目级 | `<项目根>/.workbuddy/skills/hotdownloader` | 只在这个仓库里用 |

装完的目录结构：

```
hotdownloader/
├── SKILL.md                 # 判断与路由层（agent 只默认读这一份）
├── INSTALL.md
├── CHANGELOG.md
├── references/              # 14 篇业务细节，按需加载
│   ├── architecture.md  mcp-tools.md  http-api.md
│   ├── search.md  resource-matching.md  quality-selection.md
│   ├── download-workflow.md  batch-download.md  task-management.md
│   ├── library-management.md  account-management.md
│   ├── qishui.md  signer.md  troubleshooting.md
└── tests/
    ├── validate_skill.py        # 文档工具名/HTTP 路径 ↔ Rust 源码一致性
    ├── validate_references.py   # references 结构完整性
    └── test_workflows.md        # 人工验收用例
```

配套脚本**不在 skill 目录里**，放在仓库 `scripts/`（唯一来源，避免两份副本漂移）：`qishui_login.py`、`qishui_stream.py`、`qishui_doctor.py`、`qishui-signer-server.mjs`。

## 步骤

### 1. 判断有没有现成的仓库目录

如果当前目录（或用户指定的路径）已经存在 `skills/hotdownloader/SKILL.md`，直接跳到第 3 步复制。

### 2. 取到技能文件

优先用 git（能顺手拿到配套脚本与测试）：

```bash
git clone --depth 1 https://github.com/adaier1/hotdownloaderdocker.git /tmp/hotdownloader-src
```

没有 git 时按清单逐文件下载：

```bash
base=https://raw.githubusercontent.com/adaier1/hotdownloaderdocker/main/skills/hotdownloader
dst=/tmp/hotdownloader-src/skills/hotdownloader
mkdir -p "$dst/references" "$dst/tests"

for f in SKILL.md INSTALL.md CHANGELOG.md; do
  curl -fsSL "$base/$f" -o "$dst/$f"
done
for f in architecture mcp-tools http-api search resource-matching quality-selection \
         download-workflow batch-download task-management library-management \
         account-management qishui signer troubleshooting; do
  curl -fsSL "$base/references/$f.md" -o "$dst/references/$f.md"
done
for f in validate_skill.py validate_references.py test_workflows.md; do
  curl -fsSL "$base/tests/$f" -o "$dst/tests/$f"
done
```

### 3. 复制到 skills 目录

```bash
# 用户级
mkdir -p ~/.workbuddy/skills
cp -r <源目录>/skills/hotdownloader ~/.workbuddy/skills/

# 或项目级
mkdir -p .workbuddy/skills
cp -r <源目录>/skills/hotdownloader .workbuddy/skills/
```

Windows（cmd / PowerShell 均可执行）：

```bat
xcopy <源目录>\skills\hotdownloader %USERPROFILE%\.workbuddy\skills\hotdownloader /E /I
```

### 4.（可选）放好配套脚本

只有需要"读汽水 Cookie / 自检 / 解析直链 / 起签名服务"时才用得到：

```bash
mkdir -p scripts
cp <源目录>/scripts/qishui_login.py <源目录>/scripts/qishui_doctor.py \
   <源目录>/scripts/qishui_stream.py <源目录>/scripts/qishui-signer-server.mjs scripts/
```

没有源码目录时单独下载：

```bash
base=https://raw.githubusercontent.com/adaier1/hotdownloaderdocker/main/scripts
mkdir -p scripts
for f in qishui_login.py qishui_doctor.py qishui_stream.py qishui-signer-server.mjs; do
  curl -fsSL "$base/$f" -o "scripts/$f"
done
```

### 5. 验证

```bash
ls ~/.workbuddy/skills/hotdownloader/SKILL.md
grep -m1 "^name:" ~/.workbuddy/skills/hotdownloader/SKILL.md     # 应输出 name: hotdownloader
grep -m1 "^version:" ~/.workbuddy/skills/hotdownloader/SKILL.md  # 应输出 version: 2.0.0
ls ~/.workbuddy/skills/hotdownloader/references | wc -l          # 应为 14
```

在仓库内安装时顺手跑自检（需要 `crates/` 源码，纯安装包可跳过）：

```bash
python .workbuddy/skills/hotdownloader/tests/validate_skill.py
python .workbuddy/skills/hotdownloader/tests/validate_references.py
```

### 6. 告知用户

- 装到哪个路径、是用户级还是项目级；
- **需要新开一个会话**（或刷新技能列表）才会加载新技能；
- 能力来自 HotDownloader 服务，先确认 MCP 已接入（见 `references/architecture.md` 的「MCP 连不上」）；
- 汽水功能还要装客户端并读 Cookie（`references/qishui.md`）。

## 升级

覆盖 `SKILL.md` 与 `references/`、`tests/` 即可，脚本单独更新：

```bash
cp -r <源目录>/skills/hotdownloader/. ~/.workbuddy/skills/hotdownloader/
```

变更内容看 `CHANGELOG.md`。2.0.0 相对 1.x 有接口层面的修正（任务状态、HTTP 路由、MCP/HTTP 能力差异），升级后建议清一次旧会话。

## 卸载

```bash
rm -rf ~/.workbuddy/skills/hotdownloader
```

## 说明

- 技能只提供"怎么判断、怎么调"的知识，能力来自 HotDownloader 服务的 MCP 工具与 `/api/*`；
- 技能文件不含任何凭据；示例 token 一律是占位符；
- 所有接口都在 2.0.0 校对过当前 Rust 源码，`tests/validate_skill.py` 会在文档与源码不一致时报错。
