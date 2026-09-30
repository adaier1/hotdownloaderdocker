# HotDownloader Web UI 设计规范

> 本文件整理当前 Web UI 的设计令牌、版式与组件规范，供后续修改时参考。
> 视觉风格参考 **MusicDock**：浅色、扁平、蓝色主色的卡片/胶囊/表格版式。
> 如与代码不一致，以代码为准（关键位置见每节「文件」）。

---

## 1. 技术栈与整体结构

- **框架**：Vue 3 `<script setup>` + Pinia + Vue Router（hash 模式）。
- **组件库**：Naive UI（仅浅色主题）。
- **样式策略**：
  - 全局设计令牌 + 通用组件类写在 `src/style.css`；
  - Naive 控件的颜色/圆角通过 `src/config/theme.ts` 覆盖；
  - 自定义版式（卡片、表格、胶囊、设置行）优先复用全局类，避免重复。
- **双端**：桌面 Tauri 与 Web 共用同一套前端；下载目录由运行环境决定。

### 关键文件

| 作用 | 文件 |
| --- | --- |
| 设计令牌 + 通用组件类 + 安全区 | `src/style.css` |
| Naive 主题覆盖 | `src/config/theme.ts`（经 `src/composables/useAppTheme.ts` 注入） |
| 全局 Provider / 中文 locale | `src/App.vue` |
| 侧边栏 + 底部导航 + 主内容区 | `src/components/NavLayout.vue` |
| 导航图标（内联 SVG） | `src/components/NavIcon.vue` |
| 断点（窄屏判断） | `src/composables/useNarrowLayout.ts` |
| 页面 | `src/views/*.vue` |
| 业务组件 | `src/components/**` |

---

## 2. 设计令牌（Design Tokens）

**文件**：`src/style.css` 的 `:root`。

### 2.1 颜色

| Token | 值 | 用途 |
| --- | --- | --- |
| `--bg` | `#f5f6f8` | 页面背景 |
| `--surface` / `--panel` | `#ffffff` | 卡片、侧栏、表格背景 |
| `--text-primary` | `#1f2329` | 主文本、标题 |
| `--text-secondary` | `#5f6672` | 次要文本 |
| `--text-tertiary` | `#9aa1ab` | 提示、表头、占位 |
| `--accent` | `#2b6bf3` | 主色（按钮、选中、链接） |
| `--accent-hover` | `#1f55c8` | 主色悬停 |
| `--accent-light` | `#e8f0fe` | 主色浅底（选中背景、徽标） |
| `--border` | `#e6e8eb` | 常规边框 |
| `--border-light` | `#eef0f3` | 分隔线、虚线 |
| `--border-2` | `#dde1e6` | 输入框/胶囊边框 |
| `--success` / `--green` | `#16a34a` | 成功 |
| `--green-weak` | `#e8f8ee` | 成功浅底 |
| `--warning` / `--orange` | `#d97706` | 警告 |
| `--orange-weak` | `#fdf3e3` | 警告浅底 |
| `--danger` | `#ff4d4f` | 危险、下载按钮 |
| `--danger-weak` | `#fff1f0` | 危险浅底 |
| `--purple` / `--purple-weak` | `#7c3aed` / `#f1eafe` | 辅助色 |
| `--teal` / `--teal-weak` | `#0d9488` / `#e6f7f5` | 辅助色 |

### 2.2 圆角 / 阴影 / 字体 / 动效

| Token | 值 | 用途 |
| --- | --- | --- |
| `--radius-sm` | `8px` | 输入框、按钮、小图标容器 |
| `--radius-md` | `10px` | 卡片、面板 |
| `--radius-lg` | `14px` | 大封面、移动登录卡片 |
| `--shadow-sm` | `0 1px 3px rgba(20,30,60,.04)` | 卡片默认阴影 |
| `--shadow-md` | `0 4px 16px rgba(20,30,60,.06)` | 悬浮 |
| `--shadow-lg` | `0 8px 32px rgba(20,30,60,.08)` | 弹窗、登录卡片 |
| `--transition` | `0.15s ease` | 统一过渡 |
| `--mono` | 等宽字体栈 | 路径、密钥、日志 |

字体栈：`'Noto Sans SC', -apple-system, ... , 'PingFang SC', 'Microsoft YaHei', ...`。

### 2.3 兼容别名（旧组件仍在使用）

修改令牌时只需改 `--bg/--surface/--accent` 等主值，别名会自动跟随：

```
--bg-body      → --bg
--bg-sidebar   → --surface
--bg-bottom    → --surface
--bg-content   → --bg
--border-color → --border
--color-text   → --text-primary
--color-text-secondary → --text-secondary
--color-primary → --accent
```

### 2.4 安全区

`--safe-area-top/bottom/left/right` 优先取原生插件注入变量，回退 `env()`，再回退 `0px`。
带 `html.android` 前缀的规则用于 Android 状态栏/手势条避让。

---

## 3. Naive UI 主题覆盖

**文件**：`src/config/theme.ts`，通过 `src/composables/useAppTheme.ts` 在 `App.vue` 注入。

要点（与令牌保持一致）：

- 主色 `#2b6bf3`，hover `#1f55c8`，pressed `#1a49ad`；
- 成功 `#16a34a`、警告 `#d97706`、错误 `#ff4d4f`；
- 背景 `#f5f6f8`，卡片/输入/表格 `#ffffff`，边框 `#e6e8eb`，分隔 `#eef0f3`；
- 文本 `#1f2329 / #5f6672 / #9aa1ab`；
- 输入框聚焦：`1px solid #2b6bf3` + `0 0 0 3px rgba(43,107,243,.12)`；
- 开关选中轨 `#2b6bf3`；卡片圆角 `10px`；标签圆角 `6px`。

**中文 locale**（避免组件内部出现英文按钮）：`App.vue` 的 `n-config-provider` 设置
`:locale="zhCN"` 与 `:date-locale="dateZhCN"`。

---

## 4. 排版层级

| 场景 | 字号 / 字重 |
| --- | --- |
| 页面标题 `.page-title` | 22px / 700 |
| 卡片标题 `.set-card-title` / `.sec-title` | 16px / 600 |
| 区块小标题（搜索历史/热搜） | 14.5px / 600 |
| 正文 / 表格单元 | 14px |
| 列表主标题（歌曲名） | 15~15.5px / 600 |
| 次要文本（歌手） | 13px / 常规 |
| 提示 / 表头 / 文件名副行 | 12.5px / 500（表头） |
| 徽标 `.md-badge` / 标签 `.md-tag` | 12.5px / 12px |
| 底部导航文字 | 11px |

---

## 5. 布局

**文件**：`src/components/NavLayout.vue`、`src/composables/useNarrowLayout.ts`。

- **断点**：`NARROW_LAYOUT_QUERY = '(max-width: 767px)'`（唯一来源）。
- **宽屏**：左侧固定侧栏 `220px`，右侧内容区滚动。
- **窄屏（≤767px）**：隐藏侧栏，显示底部导航，布局改为纵向。
- **页面内边距**：宽屏 `32px`（`--page-padding`），窄屏 `16px`。
- **底部导航高度**：`56px`（Android 叠加安全区）。
- **图标尺寸**：侧栏 18px，底部导航 20px。

### 侧栏 / 导航

- 侧栏背景 `--surface`，右边框 `1px --border-light`；
- 品牌图标 `32×32`、圆角 `--radius-sm`、蓝底白字；
- 导航项：`padding:10px 12px`、圆角 `9px`、`14px`；hover 背景 `#f2f3f5`；
- 选中态：背景 `--accent-light`、文字 `--accent`、字重 600（**不是**实心蓝底）。

---

## 6. 通用组件类（全局，定义在 `src/style.css`）

> 这些类在任意 `.vue` 模板中可直接使用（全局作用域，不受 `scoped` 限制）。

### 6.1 卡片与表格

| 类名 | 说明 |
| --- | --- |
| `.md-card` | 白底、`1px --border`、圆角 `--radius-md`、`--shadow-sm` |
| `.md-table-wrap` | 横向滚动容器 |
| `.md-table` | 卡片内表格：表头 12.5px/500/`--text-tertiary`、表头底 `#fafbfc`、单元 `padding:16px 18px`、行 hover `#fafbfd`、末行无下边框 |

### 6.2 胶囊 / 标签 / 徽标

| 类名 | 说明 |
| --- | --- |
| `.md-tabs` / `.md-tab` | 分类胶囊；选中 = 蓝底白字 600 |
| `.md-pills` / `.md-pill` | 筛选胶囊（任务状态） |
| `.md-chip` | 关键词胶囊（历史/热搜） |
| `.md-badge` + `.blue/.green/.gray/.red/.orange` | 状态徽标 |
| `.md-tag` + `.gold/.gray` | 音质标签；`.gold` 用于 flac/ape/无损/臻品 |
| `.md-kb-chip` | 搜索栏前缀徽标（蓝底蓝字） |
| `.md-btn-dl` | 红色下载/危险主按钮（`38px`，圆角 `9px`） |
| `.md-search` | 顶部搜索栏卡片 |

### 6.3 设置卡片与行

| 类名 | 说明 |
| --- | --- |
| `.set-card` | 设置区块卡片：`padding:22px 28px`、下间距 16px |
| `.set-card-title` | 卡片标题 16px/600 |
| `.set-row` | 设置行：`display:flex; gap:20px; padding:15px 0`，虚线分隔 |
| `.set-label` | 行标签：宽 `200px`、`14px/500`、`padding-top:8px` |
| `.set-control` / `.set-control.col` | 控件区（行 / 列） |
| `.set-hint` / `.set-hint.mono` | 说明文字（12.5px；`.mono` 等宽） |
| `.set-input` / `.set-input.mono` | 设置输入框（`38px`、圆角 8、浅底 `#fafbfc`） |
| `.set-select` | 下拉样式按钮（`min-width:150px`、高 36px） |
| `.btn-ghost` / `.btn-ghost.danger` | 描边按钮 / 红色描边按钮 |

> **设置页推荐做法**：控件行统一使用 **Naive `n-form-item`**，由 `SettingsView.vue` 的
> `:deep(.n-form-item)` 统一渲染为「左标签 200px + 右控件 + 虚线分隔」，这样所有控件起始位置一致。
> `.set-*` 系列保留给需要特殊布局的行。

---

## 7. 各页面版式要点

| 页面 | 文件 | 版式 |
| --- | --- | --- |
| 仪表盘 | `views/DashboardView.vue` | 顶部标题+连接状态徽标；左：最近下载卡片表格 + 活动日志卡片；右：统计卡列表（`.stat-card`）。失败日志完整展示失败原因（红色）。 |
| 搜索 | `views/SearchView.vue` + `components/search/*` | `.md-search` 搜索栏；`.md-tab` 类型切换；历史/热搜 `.md-chip`；歌曲行为白卡片行（封面渐变占位 + 金色音质标签 + 红色下载按钮）；歌手/专辑/歌单结果为卡片列表。 |
| 歌单 | `views/PlaylistView.vue` | 搜索栏导入；歌单信息卡片（大圆角封面）；歌曲卡片行；批量下载栏。 |
| 下载任务 | `views/TaskView.vue` + `components/task/*` | `.md-pill` 状态筛选；工具栏卡片；表格套 `.table-card`；清除操作使用模态弹窗，仅删记录不动文件。 |
| 曲库 | `views/LibraryView.vue` | 读取下载目录音频；行选择 + 单删/批量删除；表格卡片。 |
| 设置 | `views/SettingsView.vue` + `components/settings/*` | `.set-card` 分组；`n-form-item` 行；安全/MCP/通知/基本/下载设置。 |
| 关于 | `views/AboutView.vue` | 卡片 + 列表分组。 |
| 登录 | `components/WebAccessGate.vue` | 渐变背景居中卡片，品牌区 + 大输入框 + 整宽主按钮。 |

---

## 8. 图标

**文件**：`src/components/NavIcon.vue`（内联 SVG，`viewBox 0 0 24 24`，`stroke-width 2`，`currentColor`）。

支持的 `name`：`dashboard`、`search`、`playlist`、`task`、`library`、`settings`。
新增图标需同时改两处：模板分支 + `defineProps` 的联合类型。

---

## 9. 交互与无障碍

- 全局 `:focus-visible`：`2px solid --accent` + `offset 2px`。
- 动画统一 `--transition`（0.15s ease）。
- 遵循 `prefers-reduced-motion`（登录卡片/列表动画在该模式下关闭）。
- 「无数据」空态：`n-empty` 或居中提示文本，颜色 `--text-tertiary`。
- 移动端可点区域不小于 `44px`（按钮在窄屏媒体查询中抬高 min-height）。

---

## 10. 常见修改指引

### 10.1 换主色 / 品牌色
1. 改 `src/style.css` 的 `--accent / --accent-hover / --accent-light`；
2. 同步改 `src/config/theme.ts` 的 `primaryColor*`、`infoColor`、`Switch.railColorActive`，以及各 `borderFocus/boxShadowFocus` 的 rgba。
   > 注意 `rgba(43,107,243,.12)` 这类焦点阴影里的颜色也要一起改。

### 10.2 新增一个页面
1. 在 `src/views/` 新建 `XxxView.vue`；
2. `src/router/index.ts` 添加路由（需要缓存则加 `meta:{keepAlive:true}`）；
3. `src/components/NavLayout.vue` 的 `navItems` 增加项；
4. 若需新图标，`src/components/NavIcon.vue` 增加分支与类型。

### 10.3 设置页新增一行
优先使用 `n-form-item`：

```vue
<n-form-item label="标签文字">
    <!-- 控件 -->
</n-form-item>
```

说明文字放在卡片底部用 `<p class="set-hint">`，或放在控件列内。

### 10.4 ⚠️ 开关（n-switch）位置会跑偏
Naive `.n-switch` 带 `justify-content:center`。若把它放进
`flex-direction:column; align-items:stretch` 的容器，开关会被拉伸整行并居中，视觉上「太靠后」。
**避免**：用 `n-form-item`，或容器用 `align-items:flex-start` / 包一层 `display:flex` 的行容器。

### 10.5 长输入框 + 按钮同排
若输入框用 `flex:1`（基准 0），窄屏会为了和按钮同排而被压缩。需要完整显示内容时给输入框较大基准宽度：

```css
.row .set-input { flex: 1 1 480px; }  /* 空间不足时按钮换行，输入框占满一行 */
```

### 10.6 暗色模式
当前**仅浅色**（`color-scheme: light`，`theme=null`）。若需暗色，需新增 `:root` 暗色令牌与 `darkThemeOverrides`，并切换 `n-config-provider`。

---

## 11. 构建与验证

```bash
# 前端类型检查 + 构建
npx vue-tsc -b
npm run build

# Rust（本地默认若是 GNU 工具链且缺 dlltool，可用 MSVC 工具链）
cargo +stable-x86_64-pc-windows-msvc check --manifest-path crates/hotdownloader-server/Cargo.toml
cargo +stable-x86_64-pc-windows-msvc check --manifest-path src-tauri/Cargo.toml

# 本地 Docker 运行
docker compose up -d --build     # 访问 http://localhost:8080
```

---

## 12. 约束与注意点

- 全局令牌与 `theme.ts` 必须**成对维护**，否则自研组件与 Naive 控件会颜色不一致。
- `.set-control.col` 会让子元素在交叉轴拉伸；开关等固定尺寸控件勿直接放入（见 10.4）。
- 导航断点只在 `useNarrowLayout.ts` 定义，勿在组件里另写魔法值。
- 设置页控件对齐依赖 `SettingsView.vue` 里的 `:deep(.n-form-item)` 样式与 `label-width`，改动会影响整页。
- 中文文案依赖 `App.vue` 的 `zhCN` locale。
