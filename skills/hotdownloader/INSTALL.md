# 安装 hotdownloader Skill

这份文件写给 **agent**：用户把下面这句话发给你，你就照这里执行。

> 请按这个说明安装 hotdownloader Skill：`https://raw.githubusercontent.com/adaier1/hotdownloaderdocker/main/skills/hotdownloader/INSTALL.md`

## 目标

把 `skills/hotdownloader/` 装到 agent 能读到的 skills 目录，二选一：

| 级别 | 路径 | 适用 |
| --- | --- | --- |
| 用户级（推荐） | `~/.workbuddy/skills/hotdownloader`（Windows：`%USERPROFILE%\.workbuddy\skills\hotdownloader`） | 所有项目都能用 |
| 项目级 | `<项目根>/.workbuddy/skills/hotdownloader` | 只在这个仓库里用 |

装完的目录必须至少包含：

```
hotdownloader/
├── SKILL.md
└── references/
    ├── qishui.md
    └── signer.md
```

## 步骤

### 1. 判断有没有现成的仓库目录

如果当前目录（或用户指定的路径）已经存在 `skills/hotdownloader/SKILL.md`，直接跳到第 3 步复制。

### 2. 取到技能文件

优先用 git（能顺手拿到配套脚本）：

```bash
git clone --depth 1 https://github.com/adaier1/hotdownloaderdocker.git /tmp/hotdownloader-src
```

没有 git 时下载三个文件即可：

```bash
base=https://raw.githubusercontent.com/adaier1/hotdownloaderdocker/main/skills/hotdownloader
mkdir -p /tmp/hotdownloader-src/skills/hotdownloader/references
curl -fsSL "$base/SKILL.md"                    -o /tmp/hotdownloader-src/skills/hotdownloader/SKILL.md
curl -fsSL "$base/references/qishui.md"        -o /tmp/hotdownloader-src/skills/hotdownloader/references/qishui.md
curl -fsSL "$base/references/signer.md"        -o /tmp/hotdownloader-src/skills/hotdownloader/references/signer.md
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

技能里提到的脚本在仓库 `scripts/` 下，只有需要"读汽水 Cookie / 自检 / 解析直链 / 起签名服务"时才用得到：

```bash
cp <源目录>/scripts/qishui_login.py <源目录>/scripts/qishui_doctor.py \
   <源目录>/scripts/qishui_stream.py <源目录>/scripts/qishui-signer-server.mjs <项目根>/scripts/
```

没有源码目录时也可以单独下载：

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
grep -m1 "^name:" ~/.workbuddy/skills/hotdownloader/SKILL.md   # 应输出 name: hotdownloader
python scripts/qishui_doctor.py 2>/dev/null || true            # 有脚本时顺便自检
```

### 6. 告知用户

- 装到哪个路径、是用户级还是项目级；
- **需要新开一个会话**（或刷新技能列表）agent 才会加载新技能；
- 汽水相关功能还要装客户端、读 Cookie，见技能里的 `references/qishui.md`。

## 卸载

```bash
rm -rf ~/.workbuddy/skills/hotdownloader
```

## 说明

- 技能只提供"怎么用"的知识（搜索/解析/下载/排错），实际能力来自 HotDownloader 服务的 MCP 工具与 `/api/*` 接口；
- 技能文件里不含任何凭据；示例 token 一律是占位符。
