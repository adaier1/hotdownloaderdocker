<template>
    <div class="account-view" :class="{ 'is-narrow': isNarrow }">
        <div class="head-row">
            <div>
                <div class="page-title">账号设置</div>
                <div class="page-sub">统一管理 QQ 音乐、酷狗音乐与网易云音乐的登录状态</div>
            </div>
        </div>

        <!-- 三个平台的登录卡片；登录组件本身沿用设置页的原实现，保持版式一致 -->
        <div class="set-card account-section">
            <div class="set-card-title">QQ 音乐</div>
            <div class="set-card-sub">登录后可下载付费音质，并读取账号下的收藏歌单</div>
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
import { NForm } from 'naive-ui'
import { useNarrowLayout } from '../composables/useNarrowLayout'
import LoginSetting from '../components/settings/LoginSetting.vue'
import KugouLoginSetting from '../components/settings/KugouLoginSetting.vue'
import NeteaseLoginSetting from '../components/settings/NeteaseLoginSetting.vue'

const isNarrow = useNarrowLayout()
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
