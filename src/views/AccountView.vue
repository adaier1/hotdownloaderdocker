<template>
    <div class="account-view" :class="{ 'is-narrow': isNarrow }">
        <header class="page-head rise" style="--i: 0">
            <div class="page-head-main">
                <div class="eyebrow">Account · 账号</div>
                <div class="page-title">账号设置</div>
                <div class="page-sub">统一管理 QQ 音乐、酷狗音乐、网易云音乐与汽水音乐的登录状态；登录后会提高对应平台的下载音质与可用范围</div>
            </div>
        </header>

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
        <div class="set-card account-section rise" style="--i: 2">
            <div class="set-card-title">QQ 音乐</div>
            <div class="set-card-sub">登录后可下载付费音质，并读取账号下的收藏歌单；未登录时部分歌曲会拿不到下载链接</div>
            <n-form :label-placement="isNarrow ? 'top' : 'left'">
                <LoginSetting />
            </n-form>
        </div>

        <div class="set-card account-section rise" style="--i: 3">
            <div class="set-card-title">酷狗音乐</div>
            <div class="set-card-sub">登录后可下载高音质资源，并读取账号下的歌单</div>
            <n-form :label-placement="isNarrow ? 'top' : 'left'">
                <KugouLoginSetting />
            </n-form>
        </div>

        <div class="set-card account-section rise" style="--i: 4">
            <div class="set-card-title">网易云音乐</div>
            <div class="set-card-sub">登录后可下载高音质资源，并读取账号下的歌单</div>
            <n-form :label-placement="isNarrow ? 'top' : 'left'">
                <NeteaseLoginSetting />
            </n-form>
        </div>

        <div class="set-card account-section rise" style="--i: 5">
            <div class="set-card-title">汽水音乐</div>
            <div class="set-card-sub">
                手动粘贴登录 Cookie 后，搜索与详情才会带上登录态；未配置时只能用分享链接解析，会员曲目只有试听片段
            </div>
            <n-form :label-placement="isNarrow ? 'top' : 'left'">
                <QishuiCookieSetting />
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
import QishuiCookieSetting from '../components/settings/QishuiCookieSetting.vue'
import { getLoginStatus } from '../api/musicApi'
import { kugouLogin } from '../api/kugouLogin'
import { neteaseLogin } from '../api/neteaseLogin'
import { getQishuiCookieStatus, getQishuiSignerStatus } from '../api/qishuiApi'

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
    { key: 'qishui', name: '汽水音乐', state: 'loading', detail: '' },
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
        const [qq, kugou, netease, qishui, signer] = await Promise.all([
            getLoginStatus('qqmusic').catch(() => null),
            kugouLogin('status').catch(() => null),
            neteaseLogin('status').catch(() => null),
            getQishuiCookieStatus().catch(() => null),
            getQishuiSignerStatus().catch(() => null),
        ])
        mark('qqmusic', qq?.logged_in ? 'ok' : 'none', qq?.uin ?? '')
        mark('kugou', kugou?.logged_in ? 'ok' : 'none', kugou?.userid ?? '')
        mark('netease', netease?.logged_in ? 'ok' : 'none', netease?.nickname ?? '')
        // 汽水没有账号信息可展示；签名是否就绪决定搜索与高音质能不能用。
        mark(
            'qishui',
            qishui?.configured ? 'ok' : 'none',
            qishui?.configured ? (signer?.available ? 'Cookie + 签名' : 'Cookie 已配置') : '',
        )
    } catch {
        mark('qqmusic', 'error')
        mark('kugou', 'error')
        mark('netease', 'error')
        mark('qishui', 'error')
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
    max-width: 980px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    min-height: 100%;
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
    border-bottom: 1px solid var(--border-light);
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

/* ---------- 登录控件与整站语言对齐 ---------- */

/* 扫码/手动登录：从下划线标签改成灰槽分段控件 */
.account-view :deep(.n-tabs-nav) {
    max-width: fit-content;
    margin-bottom: 10px;
}

.account-view :deep(.n-tabs-nav-scroll) {
    display: inline-flex;
}

.account-view :deep(.n-tabs-nav--line-type .n-tabs-nav-scroll-content) {
    display: inline-flex;
    gap: 2px;
    padding: 4px;
    border-bottom: none !important;
    background: var(--surface-sunken);
    border: 1px solid var(--border-light);
    border-radius: 999px;
}

.account-view :deep(.n-tabs-nav--line-type .n-tabs-nav-scroll-content::before) {
    display: none;
}

.account-view :deep(.n-tabs-tab) {
    padding: 6px 16px !important;
    margin: 0 !important;
    border-radius: 999px;
    font-size: 12.5px;
    font-weight: 450;
    color: var(--text-secondary);
    transition: color var(--transition), background var(--transition);
}

.account-view :deep(.n-tabs-tab:hover) {
    color: var(--text-primary) !important;
}

.account-view :deep(.n-tabs-tab.n-tabs-tab--active) {
    background: var(--accent);
    color: var(--accent-contrast) !important;
    font-weight: 500;
}

.account-view :deep(.n-tabs-bar) {
    display: none;
}

.account-view :deep(.n-tabs-pane-wrapper) {
    padding-top: 4px;
}

/* 二维码：给一块有边界的扫码区，避免未出图时留下大片空白 */
.account-view :deep(.qr-container) {
    min-height: 190px;
    justify-content: center;
    padding: 18px;
    border: 1px dashed var(--border-2);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
}

.account-view :deep(.qr-tips) {
    font-size: 12px;
    color: var(--text-tertiary);
}

.account-view :deep(.qr-container .n-button) {
    border-radius: 999px;
    height: 32px;
    font-size: 12.5px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-secondary);
}

/* 提交类按钮：不再整行铺满，收成胶囊 */
.account-view :deep(.n-form .n-button.n-button--primary-type) {
    border-radius: 999px;
    max-width: 240px;
    height: 38px;
    font-size: 13px;
}

/* 退出登录：从红色实心改为安静描边 */
.account-view :deep(.logged-in .n-button.n-button--error-type) {
    border-radius: 999px;
    background: transparent !important;
    color: var(--danger) !important;
    border: 1px solid #e8d2cd !important;
    height: 30px;
    font-size: 12.5px;
}

.account-view :deep(.logged-in .n-button.n-button--error-type:hover) {
    background: var(--danger-weak) !important;
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
