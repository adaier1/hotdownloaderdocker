<template>
    <div class="account-view" :class="{ 'is-narrow': isNarrow }">
        <div class="head-row">
            <div>
                <div class="page-title">账号设置</div>
                <div class="page-sub">统一管理 QQ 音乐、酷狗音乐与网易云音乐的登录状态；登录后会提高对应平台的下载音质与可用范围</div>
            </div>
        </div>

        <!-- 登录状态总览：换源会跨平台，先让用户看清哪个平台还没登录 -->
        <div class="status-bar">
            <div v-for="item in statuses" :key="item.key" class="status-chip" :class="'is-' + item.state">
                <span class="status-dot" />
                <span class="status-name">{{ item.name }}</span>
                <span class="status-value">{{ statusText(item) }}</span>
            </div>
            <n-button text size="small" :loading="refreshing" @click="refreshStatus">刷新状态</n-button>
        </div>

        <!-- 三个平台的登录卡片；登录组件本身沿用设置页的原实现，保持版式一致 -->
        <div class="set-card account-section">
            <div class="set-card-title">QQ 音乐</div>
            <div class="set-card-sub">登录后可下载付费音质，并读取账号下的收藏歌单；未登录时部分歌曲会拿不到下载链接</div>
            <n-form :label-placement="isNarrow ? 'top' : 'left'">
                <LoginSetting />
            </n-form>
        </div>

        <div class="set-card account-section">
            <div class="set-card-title">酷狗音乐</div>
            <div class="set-card-sub">登录后可下载高音质资源，并读取账号下的歌单</div>
            <n-form :label-placement="isNarrow ? 'top' : 'left'">
                <KugouLoginSetting />
            </n-form>
        </div>

        <div class="set-card account-section">
            <div class="set-card-title">网易云音乐</div>
            <div class="set-card-sub">登录后可下载高音质资源，并读取账号下的歌单</div>
            <n-form :label-placement="isNarrow ? 'top' : 'left'">
                <NeteaseLoginSetting />
            </n-form>
        </div>
    </div>
</template>

<script setup lang="ts">
import { onActivated, onMounted, ref } from 'vue'
import { NButton, NForm } from 'naive-ui'
import { useNarrowLayout } from '../composables/useNarrowLayout'
import LoginSetting from '../components/settings/LoginSetting.vue'
import KugouLoginSetting from '../components/settings/KugouLoginSetting.vue'
import NeteaseLoginSetting from '../components/settings/NeteaseLoginSetting.vue'
import { getLoginStatus } from '../api/musicApi'
import { kugouLogin } from '../api/kugouLogin'
import { neteaseLogin } from '../api/neteaseLogin'

type LoginState = 'loading' | 'ok' | 'none' | 'guest' | 'error'

interface PlatformStatus {
    key: string
    name: string
    state: LoginState
    detail: string
}

const isNarrow = useNarrowLayout()
const refreshing = ref(false)

// 酷我没有登录机制，匿名即可下载 128k，因此单独标成"无需登录"。
const statuses = ref<PlatformStatus[]>([
    { key: 'qqmusic', name: 'QQ 音乐', state: 'loading', detail: '' },
    { key: 'kugou', name: '酷狗音乐', state: 'loading', detail: '' },
    { key: 'netease', name: '网易云音乐', state: 'loading', detail: '' },
    { key: 'kuwo', name: '酷我音乐', state: 'guest', detail: '' },
])

function statusText(item: PlatformStatus): string {
    switch (item.state) {
        case 'ok':
            return item.detail || '已登录'
        case 'none':
            return '未登录'
        case 'guest':
            return '匿名可用'
        case 'error':
            return '查询失败'
        default:
            return '检查中'
    }
}

async function refreshStatus() {
    refreshing.value = true
    const mark = (key: string, state: LoginState, detail = '') => {
        const item = statuses.value.find((entry) => entry.key === key)
        if (item) {
            item.state = state
            item.detail = detail
        }
    }
    try {
        const [qq, kugou, netease] = await Promise.all([
            getLoginStatus('qqmusic').catch(() => null),
            kugouLogin('status').catch(() => null),
            neteaseLogin('status').catch(() => null),
        ])
        mark('qqmusic', qq?.logged_in ? 'ok' : 'none', qq?.uin ?? '')
        mark('kugou', kugou?.logged_in ? 'ok' : 'none', kugou?.userid ?? '')
        mark('netease', netease?.logged_in ? 'ok' : 'none', netease?.nickname ?? '')
    } catch {
        mark('qqmusic', 'error')
        mark('kugou', 'error')
        mark('netease', 'error')
    } finally {
        refreshing.value = false
    }
}

onMounted(refreshStatus)
// keepAlive 下从别的页面回到账号页时重新核对，登录/退出后可立即看到变化。
onActivated(refreshStatus)
</script>

<style scoped>
.account-view {
    width: 100%;
    max-width: none;
    min-width: 0;
    display: flex;
    flex-direction: column;
    min-height: 100%;
}

.head-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 20px;
}

.page-title {
    font-size: 22px;
    font-weight: 700;
    color: var(--text-primary);
}

.page-sub {
    font-size: 13px;
    color: var(--text-secondary);
    margin-top: 4px;
}

.status-bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 16px;
}

.status-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 12.5px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
}

.status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-tertiary);
    flex-shrink: 0;
}

.status-chip.is-ok .status-dot {
    background: var(--success);
}

.status-chip.is-none .status-dot {
    background: var(--warning);
}

.status-chip.is-error .status-dot {
    background: var(--danger);
}

.status-name {
    color: var(--text-primary);
    font-weight: 500;
}

.status-value {
    color: var(--text-tertiary);
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.account-view.is-narrow {
    max-width: none;
}

.account-view :deep(.set-card-title) {
    margin-bottom: 4px;
}

.set-card-sub {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.6;
    margin-bottom: 6px;
}

.account-section {
    margin-bottom: 16px;
}

/* 与设置页一致：把 Naive 表单行渲染为 MusicDock 的 set-row 版式 */
.account-view :deep(.n-form-item) {
    padding: 14px 0;
    margin-bottom: 0;
    border-bottom: 1px dashed var(--border-light);
}

.account-view :deep(.n-form-item:last-child) {
    border-bottom: none;
}

.account-view :deep(.n-form-item-label) {
    font-weight: 500;
    color: var(--text-primary);
    text-align: left;
    justify-content: flex-start;
}

.account-view :deep(.n-form-item-blank),
.account-view :deep(.n-input-group) {
    min-width: 0;
}

/* 多行输入（如网易云手动登录的 Cookie）在收缩型 flex 容器中会塌陷成一条细线，给出最小宽度 */
.account-view :deep(.n-input--textarea) {
    min-width: 360px;
}

@media (min-width: 768px) {
    .account-view :deep(.n-form-item) {
        display: flex;
        align-items: flex-start;
        gap: 20px;
    }

    .account-view :deep(.n-form-item-label) {
        padding-top: 8px;
        padding-bottom: 0;
    }
}

@media (max-width: 767px) {
    .set-card {
        padding: 16px 14px;
    }

    .account-view :deep(.n-form-item) {
        padding: 12px 0;
    }

    .account-view :deep(.n-button) {
        min-height: 44px;
    }

    /* 窄屏标签在输入框上方，输入框自然撑满，取消桌面端的最小宽度约束 */
    .account-view :deep(.n-input--textarea) {
        min-width: 0;
    }
}
</style>
