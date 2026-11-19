<template>
    <div class="nav-layout" :class="{ 'is-narrow': isNarrow }">
        <!-- 宽屏左侧垂直导航 -->
        <aside v-if="!isNarrow" class="sidebar">
            <div class="brand">
                <div class="brand-mark">H</div>
                <div class="brand-text">
                    <span class="brand-name">HotDownloader</span>
                    <span class="brand-sub">音乐下载工作台</span>
                </div>
            </div>

            <nav class="nav-groups" aria-label="主导航">
                <div class="nav-group">
                    <div class="nav-group-label">工作台</div>
                    <button v-for="item in primaryItems" :key="item.key" class="nav-item"
                        :class="{ active: currentRoute === item.key }" type="button"
                        @click="handleMenuClick(item.key)">
                        <NavIcon :name="item.icon" class="nav-item-icon" />
                        <span class="nav-item-label">{{ item.label }}</span>
                        <span v-if="badgeOf(item.key)" class="nav-item-count">{{ badgeOf(item.key) }}</span>
                    </button>
                </div>

                <div class="nav-group">
                    <div class="nav-group-label">账户</div>
                    <button v-for="item in secondaryItems" :key="item.key" class="nav-item"
                        :class="{ active: currentRoute === item.key }" type="button"
                        @click="handleMenuClick(item.key)">
                        <NavIcon :name="item.icon" class="nav-item-icon" />
                        <span class="nav-item-label">{{ item.label }}</span>
                    </button>
                </div>
            </nav>

            <div class="sidebar-footer">
                <div class="conn" :class="connectionClass">
                    <span class="conn-dot" />
                    {{ connectionText }}
                </div>
                <div class="footer-row">
                    <span class="version">v{{ version }}</span>
                    <button v-if="!native" class="logout-button" type="button" @click="confirmLogout">退出登录</button>
                </div>
            </div>
        </aside>

        <!-- 内容区域 -->
        <main ref="mainContentRef" class="main-content" :class="{ 'has-bottom-nav': isNarrow }">
            <div class="page-canvas">
                <router-view v-slot="{ Component }">
                    <!-- 每个详情独立缓存，返回时恢复其分页、标签与勾选。 -->
                    <keep-alive>
                        <component :is="Component" :key="viewKey" />
                    </keep-alive>
                </router-view>
            </div>
        </main>

        <!-- 窄屏底部水平导航：在正常文档流中固定占位 -->
        <footer v-if="isNarrow" class="bottom-nav">
            <div class="bottom-nav-inner">
                <button v-for="item in navItems" :key="item.key" class="bottom-nav-item"
                    :class="{ active: currentRoute === item.key }" type="button" @click="handleMenuClick(item.key)">
                    <NavIcon :name="item.icon" class="bottom-nav-icon" />
                    <span>{{ item.shortLabel || item.label }}</span>
                </button>
            </div>
        </footer>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useDialog, useNotification } from 'naive-ui'
import NavIcon from './NavIcon.vue'
import { useCloseGuard } from '../composables/useCloseGuard'
import { useNarrowLayout } from '../composables/useNarrowLayout'
import { isNativeRuntime } from '../api/runtimeApi'
import { logoutWeb } from '../api/webClient'
import { useTaskStore } from '../stores/taskStore'
import { useLibraryStore } from '../stores/libraryStore'

const router = useRouter()
const route = useRoute()
const version = import.meta.env.VITE_APP_VERSION
const viewKey = computed(() => {
    return ['/artist', '/album'].includes(route.path) ? route.fullPath : route.path
})

// 保存各路由页面的滚动位置，实现独立滚动记录
const mainContentRef = ref<HTMLElement | null>(null)
const scrollPositions: Record<string, number> = {}

let removeRouteGuard: (() => void) | null = null

// 恢复指定路由的滚动位置
async function restoreScrollPosition(path: string) {
    await nextTick()
    if (mainContentRef.value) {
        mainContentRef.value.scrollTop = scrollPositions[path] ?? 0
    }
}

// 监听路由变化，恢复新路由的滚动位置
watch(() => route.fullPath, (newPath) => {
    restoreScrollPosition(newPath)
})

// 在 n-dialog-provider 内部调用，确保 useDialog 正常工作
useCloseGuard()

// 挂载通知实例到全局，供 store 使用
const notification = useNotification()
window.$notify = notification

// Web 版登录退出（桌面端不经过 HTTP 认证）
const native = isNativeRuntime()
const dialog = useDialog()

function confirmLogout() {
    dialog.warning({
        title: '退出登录',
        content: '确定要退出当前登录状态吗？退出后需重新输入密码。',
        positiveText: '退出',
        negativeText: '取消',
        onPositiveClick: () => logoutWeb(),
    })
}

// 移动端响应式布局状态；公共方法统一断点，并在组件销毁时清理监听。
const isNarrow = useNarrowLayout()

// 侧栏里的实时计数：只读，不触发额外请求
const taskStore = useTaskStore()
const libraryStore = useLibraryStore()

function badgeOf(key: string): number {
    if (key === '/task') return taskStore.tasks.length
    if (key === '/library') return libraryStore.files.length
    return 0
}

const connectionText = computed(() => {
    switch (taskStore.connectionStatus) {
        case 'connected': return '已连接'
        case 'connecting': return '连接中'
        case 'reconnecting': return '重连中'
        default: return '离线'
    }
})

const connectionClass = computed(() => ({
    online: taskStore.connectionStatus === 'connected',
    offline: taskStore.connectionStatus === 'disconnected',
}))

onMounted(() => {
    // 注册全局前置守卫，在离开当前路由前保存滚动位置
    removeRouteGuard = router.beforeEach((_to, from) => {
        if (mainContentRef.value) {
            scrollPositions[from.fullPath] = mainContentRef.value.scrollTop
        }
    })
    // 初始恢复当前路由的滚动位置（如果有保存过）
    restoreScrollPosition(route.fullPath)
})

onUnmounted(() => {
    // 移除路由守卫，避免内存泄漏
    if (removeRouteGuard) {
        removeRouteGuard()
        removeRouteGuard = null
    }
})

// 关于页属于设置入口，返回时继续保持设置菜单高亮。
const currentRoute = computed(() => {
    if (route.path === '/album' || route.path === '/artist') return '/search'
    return route.path.startsWith('/settings/') ? '/settings' : route.path
})

type NavItem = {
    key: string
    label: string
    /** 窄屏底部导航使用的短标签，缺省时沿用 label */
    shortLabel?: string
    icon: 'dashboard' | 'search' | 'playlist' | 'task' | 'library' | 'logs' | 'account' | 'settings'
}

const primaryItems: NavItem[] = [
    { key: '/dashboard', label: '总览', icon: 'dashboard' },
    { key: '/search', label: '搜索', icon: 'search' },
    { key: '/playlist', label: '解析', icon: 'playlist' },
    { key: '/task', label: '下载任务', icon: 'task' },
    { key: '/library', label: '曲库', icon: 'library' },
    { key: '/logs', label: '日志', icon: 'logs' },
]

const secondaryItems: NavItem[] = [
    { key: '/account', label: '账号', shortLabel: '账号', icon: 'account' },
    { key: '/settings', label: '设置', icon: 'settings' },
]

const navItems: NavItem[] = [...primaryItems, ...secondaryItems]

function handleMenuClick(key: string) {
    if (key !== route.path) {
        router.push(key)
    }
}
</script>

<style scoped>
/* 布局整体 */
.nav-layout {
    --page-padding: 30px;

    display: flex;
    height: 100%;
    min-height: 0;
    background: var(--bg);
}

.nav-layout.is-narrow {
    --page-padding: 16px;

    flex-direction: column;
}

/* ============ 侧边栏 ============ */
.sidebar {
    width: 232px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    padding: 22px 14px 14px;
    background-color: var(--bg-sidebar);
    overflow-y: auto;

    /* 横屏时让导航避开状态栏和侧边安全区。 */
    padding-top: calc(22px + var(--safe-area-top));
    padding-left: calc(14px + var(--safe-area-left));
}

.brand {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 2px 8px 26px;
}

.brand-mark {
    width: 30px;
    height: 30px;
    flex-shrink: 0;
    border-radius: 9px;
    background: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 14px;
    letter-spacing: 0.01em;
}

.brand-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
}

.brand-name {
    font-size: 13.5px;
    font-weight: 600;
    letter-spacing: -0.015em;
    color: var(--text-primary);
    line-height: 1.2;
}

.brand-sub {
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-tertiary);
}

.nav-groups {
    display: flex;
    flex-direction: column;
    gap: 22px;
}

.nav-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.nav-group-label {
    font-family: var(--mono);
    font-size: 9.5px;
    font-weight: 500;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    padding: 0 10px 8px;
}

.nav-item {
    display: flex;
    align-items: center;
    gap: 11px;
    height: 38px;
    padding: 0 10px;
    border-radius: 10px;
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 13.5px;
    font-weight: 450;
    font-family: inherit;
    transition: background var(--transition), color var(--transition), box-shadow var(--transition);
    border: 1px solid transparent;
    background: transparent;
    width: 100%;
    text-align: left;
}

.nav-item:hover {
    background: rgba(255, 255, 255, 0.72);
    color: var(--text-primary);
}

.nav-item.active {
    background: var(--surface);
    border-color: var(--border);
    color: var(--text-primary);
    font-weight: 550;
    box-shadow: 0 1px 2px rgba(28, 28, 22, 0.05);
}

.nav-item-icon {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    opacity: 0.75;
}

.nav-item.active .nav-item-icon {
    opacity: 1;
    color: var(--accent);
}

.nav-item-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.nav-item-count {
    flex: none;
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
}

.nav-item.active .nav-item-count {
    color: var(--text-secondary);
}

.sidebar-footer {
    margin-top: auto;
    padding: 14px 8px 4px;
    border-top: 1px solid var(--border-light);
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.conn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 11.5px;
    color: var(--text-tertiary);
}

.conn-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--text-tertiary);
}

.conn.online {
    color: var(--green);
}

.conn.online .conn-dot {
    background: var(--green);
    box-shadow: 0 0 0 3px rgba(77, 122, 88, 0.14);
}

.conn.offline {
    color: var(--danger);
}

.conn.offline .conn-dot {
    background: var(--danger);
}

.footer-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
}

.version {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-tertiary);
    letter-spacing: 0.04em;
}

.logout-button {
    border: none;
    background: none;
    padding: 3px 8px;
    margin-right: -8px;
    border-radius: 6px;
    color: var(--text-tertiary);
    font: inherit;
    font-size: 11.5px;
    cursor: pointer;
    transition: color var(--transition), background var(--transition);
    white-space: nowrap;
}

.logout-button:hover {
    background: rgba(28, 28, 22, 0.05);
    color: var(--danger);
}

/* ============ 主内容 ============ */
.main-content {
    flex: 1;
    /* 允许内容随窗口收缩，避免长列表撑开整个布局。 */
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    background-color: var(--bg-content);

    /* 顶部一层极淡的暖光，让纸面不平 */
    background-image: radial-gradient(1100px 420px at 18% -10%, rgba(255, 255, 255, 0.85) 0%, rgba(255, 255, 255, 0) 68%);

    /* 用四个方向分别声明，让左右两侧都避开安全区 */
    padding: var(--page-padding);
    padding-left: calc(var(--page-padding) + var(--safe-area-left));
    padding-right: calc(var(--page-padding) + var(--safe-area-right));

    /* 将回弹限制在当前滚动容器内部，保留视觉回弹但阻断滚动链向上传播 */
    overscroll-behavior: contain;
}

.page-canvas {
    max-width: 1240px;
    margin: 0 auto;
    min-height: 100%;
    display: flex;
    flex-direction: column;
}

/* 为正常流底部导航保留合适的底部间距，避免内容与导航粘连 */
.main-content.has-bottom-nav {
    padding-bottom: 16px;
    padding-left: calc(16px + var(--safe-area-left));
    padding-right: calc(16px + var(--safe-area-right));
}

/* ============ 底部导航（窄屏） ============ */
.bottom-nav {
    width: 100%;
    height: 56px;
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    background-color: var(--bg-bottom);
    padding-left: var(--safe-area-left);
    padding-right: var(--safe-area-right);
}

.bottom-nav-inner {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: stretch;
    gap: 2px;
    padding: 6px 8px;
}

.bottom-nav-item {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    min-height: 44px;
    border: none;
    border-radius: 10px;
    background: none;
    cursor: pointer;
    font-family: inherit;
    font-size: 10.5px;
    font-weight: 500;
    color: var(--text-tertiary);
    transition: color var(--transition), background var(--transition);
}

.bottom-nav-item.active {
    color: var(--accent);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
}

/* 导航项增加到 7 个后，长中文标签在窄屏不换行，避免撑破底部导航 */
.bottom-nav-item span {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.bottom-nav-icon {
    width: 18px;
    height: 18px;
}
</style>
