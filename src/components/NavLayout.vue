<template>
    <div class="nav-layout" :class="{ 'is-narrow': isNarrow }">
        <!-- 宽屏左侧垂直导航 -->
        <aside v-if="!isNarrow" class="sidebar">
            <div class="sidebar-logo">
                <div class="sidebar-logo-icon">H</div>
                <div class="sidebar-logo-text">HotDownloader</div>
            </div>
            <nav class="nav-list">
                <button v-for="item in navItems" :key="item.key" class="nav-item"
                    :class="{ active: currentRoute === item.key }" type="button" @click="handleMenuClick(item.key)">
                    <NavIcon :name="item.icon" class="nav-item-icon" />
                    <span>{{ item.label }}</span>
                </button>
            </nav>
            <div class="sidebar-footer">
                <span>v{{ version }}</span>
                <button v-if="!native" class="logout-button" type="button" @click="confirmLogout">退出登录</button>
            </div>
        </aside>

        <!-- 内容区域 -->
        <main ref="mainContentRef" class="main-content" :class="{ 'has-bottom-nav': isNarrow }">
            <router-view v-slot="{ Component }">
                <!-- 每个详情独立缓存，返回时恢复其分页、标签与勾选。 -->
                <keep-alive>
                    <component :is="Component" :key="viewKey" />
                </keep-alive>
            </router-view>
        </main>

        <!-- 窄屏底部水平导航：在正常文档流中固定占位 -->
        <footer v-if="isNarrow" class="bottom-nav">
            <div class="bottom-nav-inner">
                <button v-for="item in navItems" :key="item.key" class="bottom-nav-item"
                    :class="{ active: currentRoute === item.key }" type="button" @click="handleMenuClick(item.key)">
                    <NavIcon :name="item.icon" class="bottom-nav-icon" />
                    <span>{{ item.label }}</span>
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

const navItems = [
    { key: '/dashboard', label: '仪表盘', icon: 'dashboard' as const },
    { key: '/search', label: '搜索', icon: 'search' as const },
    { key: '/playlist', label: '歌单', icon: 'playlist' as const },
    { key: '/task', label: '下载任务', icon: 'task' as const },
    { key: '/library', label: '曲库', icon: 'library' as const },
    { key: '/settings', label: '设置', icon: 'settings' as const },
]

function handleMenuClick(key: string) {
    if (key !== route.path) {
        router.push(key)
    }
}
</script>

<style scoped>
/* 布局整体 */
.nav-layout {
    --page-padding: 32px;

    display: flex;
    height: 100%;
    min-height: 0;
}

.nav-layout.is-narrow {
    --page-padding: 16px;

    flex-direction: column;
}

/* 侧边栏 */
.sidebar {
    width: 220px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border-light);
    padding: 20px 12px;
    background-color: var(--surface);
    overflow-y: auto;

    /* 横屏时让导航避开状态栏和侧边安全区。 */
    padding-top: calc(20px + var(--safe-area-top));
    padding-left: calc(12px + var(--safe-area-left));
}

.sidebar-logo {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px 24px;
}

.sidebar-logo-icon {
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #fff;
    font-weight: 700;
    font-size: 16px;
}

.sidebar-logo-text {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
}

.nav-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
}

.nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 9px;
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 500;
    font-family: inherit;
    transition: background var(--transition), color var(--transition);
    border: none;
    background: none;
    width: 100%;
    text-align: left;
}

.nav-item:hover {
    background: #f2f3f5;
    color: var(--text-primary);
}

.nav-item.active {
    background: var(--accent-light);
    color: var(--accent);
    font-weight: 600;
}

.nav-item-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
}

.sidebar-footer {
    margin-top: auto;
    padding: 12px 8px 0;
    font-size: 11px;
    color: var(--text-tertiary);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
}

.logout-button {
    border: none;
    background: none;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font: inherit;
    cursor: pointer;
    transition: all var(--transition);
    white-space: nowrap;
}

.logout-button:hover {
    background: rgba(0, 0, 0, 0.04);
    color: var(--danger);
}

/* 主内容区背景 */
.main-content {
    flex: 1;
    /* 允许内容随窗口收缩，避免长列表撑开整个布局。 */
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    background-color: var(--bg-content);

    /* 用四个方向分别声明，让左右两侧都避开安全区 */
    padding: var(--page-padding);
    padding-left: calc(var(--page-padding) + var(--safe-area-left));
    padding-right: calc(var(--page-padding) + var(--safe-area-right));

    /* 将回弹限制在当前滚动容器内部，保留视觉回弹但阻断滚动链向上传播 */
    overscroll-behavior: contain;
}

/* 为正常流底部导航保留合适的底部间距，避免内容与导航粘连 */
.main-content.has-bottom-nav {
    padding-bottom: 16px;
    padding-left: calc(16px + var(--safe-area-left));
    padding-right: calc(16px + var(--safe-area-right));
}

/* 底部导航：正常流布局，高度由 flex 分配 */
.bottom-nav {
    width: 100%;
    height: 56px;
    flex-shrink: 0;
    border-top: 1px solid var(--border-light);
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
    background: none;
    cursor: pointer;
    font-family: inherit;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    transition: color var(--transition);
}

.bottom-nav-item.active {
    color: var(--accent);
}

.bottom-nav-icon {
    width: 20px;
    height: 20px;
}
</style>
