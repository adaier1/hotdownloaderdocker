<template>
    <div class="login-setting">
        <!-- 未登录状态 -->
        <template v-if="!isLoggedIn">
            <n-tabs v-model:value="activeTab" type="line" size="small">
                <n-tab-pane name="qr" tab="扫码登录">
                    <div class="qr-container">
                        <img v-if="qrImage" :src="qrImage" alt="二维码" class="qr-img" />
                        <div class="qr-tips">
                            <span v-if="qrStatus === 'waiting'">请使用网易云音乐 App 扫码</span>
                            <span v-else-if="qrStatus === 'scanned'">已扫码，请在手机上确认</span>
                            <span v-else-if="qrStatus === 'expired'">二维码已过期，请刷新</span>
                            <span v-else-if="qrStatus === 'error'">登录出错，请重试</span>
                        </div>
                        <n-button size="small" @click="refreshQr" :loading="qrLoading">刷新二维码</n-button>
                    </div>
                </n-tab-pane>
                <n-tab-pane name="manual" tab="手动登录">
                    <n-form :label-placement="isNarrow ? 'top' : 'left'" :label-width="isNarrow ? undefined : 110">
                        <n-form-item label="Cookie">
                            <n-input
                                v-model:value="manualCookie"
                                type="textarea"
                                :autosize="{ minRows: 3, maxRows: 6 }"
                                placeholder="粘贴网页版登录 Cookie，需包含 MUSIC_U"
                            />
                        </n-form-item>
                        <n-button type="primary" block @click="handleManualLogin" :loading="manualLoading">
                            登录
                        </n-button>
                    </n-form>
                </n-tab-pane>
            </n-tabs>
        </template>

        <!-- 已登录状态 -->
        <template v-else>
            <div class="logged-in">
                <span>已登录：{{ nickname || userId }}</span>
                <div class="logged-in-actions">
                    <n-button size="small" @click="handleRefresh" :loading="refreshLoading">刷新登录</n-button>
                    <n-button size="small" type="error" @click="handleLogout">退出登录</n-button>
                </div>
            </div>
        </template>
    </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { NTabs, NTabPane, NForm, NFormItem, NInput, NButton, useNotification } from 'naive-ui'
import { neteaseLogin, type NeteaseLoginResult } from '../../api/neteaseLogin'
import { useNarrowLayout } from '../../composables/useNarrowLayout'

const notification = useNotification()
const isNarrow = useNarrowLayout()
const isLoggedIn = ref(false)
const userId = ref('')
const nickname = ref('')
const activeTab = ref('manual')

// 二维码相关
const qrImage = ref('')
const qrId = ref('')
const qrStatus = ref('waiting')
const qrLoading = ref(false)
let pollTimer: ReturnType<typeof setInterval> | null = null

// 手动登录
const manualCookie = ref('')
const manualLoading = ref(false)
const refreshLoading = ref(false)

async function refreshQr() {
    if (pollTimer) clearInterval(pollTimer)
    qrLoading.value = true
    try {
        const res = await neteaseLogin('qr')
        qrImage.value = res.qr_image ?? ''
        qrId.value = res.qrcode_id ?? ''
        qrStatus.value = 'waiting'
        startPolling()
    } catch (error) {
        console.error('获取网易云二维码失败:', error)
        qrStatus.value = 'error'
        notification.error({ content: `获取网易云二维码失败：${errorMessage(error)}`, duration: 3000 })
    } finally {
        qrLoading.value = false
    }
}

function errorMessage(error: unknown): string {
    return error instanceof Error && error.message ? error.message : String(error)
}

function applyStatus(result: NeteaseLoginResult) {
    isLoggedIn.value = result.logged_in ?? false
    userId.value = result.userid ?? userId.value
    nickname.value = result.nickname ?? nickname.value
}

function startPolling() {
    if (pollTimer) clearInterval(pollTimer)
    pollTimer = setInterval(async () => {
        try {
            const result = await neteaseLogin('check', qrId.value)
            qrStatus.value = result.status ?? 'error'
            if (result.status === 'confirmed') {
                if (pollTimer) clearInterval(pollTimer)
                applyStatus(result)
            } else if (result.status === 'expired' || result.status === 'error') {
                if (pollTimer) clearInterval(pollTimer)
            }
        } catch (error) {
            console.error('轮询网易云登录状态失败:', error)
            qrStatus.value = 'error'
            notification.error({ content: `登录状态检查失败：${errorMessage(error)}`, duration: 3000 })
            if (pollTimer) clearInterval(pollTimer)
        }
    }, 2000)
}

async function handleManualLogin() {
    if (!manualCookie.value) {
        notification.error({ content: 'Cookie 为必填项，需包含 MUSIC_U', duration: 3000 })
        return
    }
    manualLoading.value = true
    try {
        const result = await neteaseLogin('manual', '', { cookie: manualCookie.value })
        applyStatus(result)
    } catch (error) {
        console.error('网易云手动登录失败:', error)
        notification.error({ content: `登录失败：${errorMessage(error)}`, duration: 3000 })
    } finally {
        manualLoading.value = false
    }
}

async function handleRefresh() {
    refreshLoading.value = true
    try {
        const result = await neteaseLogin('refresh')
        applyStatus(result)
    } catch (error) {
        console.error('刷新网易云登录失败:', error)
        notification.error({ content: `刷新失败：${errorMessage(error)}`, duration: 3000 })
    } finally {
        refreshLoading.value = false
    }
}

async function handleLogout() {
    try {
        await neteaseLogin('logout')
        isLoggedIn.value = false
        userId.value = ''
        nickname.value = ''
        if (pollTimer) clearInterval(pollTimer)
    } catch (error) {
        console.error('退出网易云登录失败:', error)
    }
}

onMounted(async () => {
    try {
        const status = await neteaseLogin('status')
        applyStatus(status)
    } catch (error) {
        console.error('获取网易云登录状态失败:', error)
    }
})

watch(activeTab, async (tab) => {
    if (tab === 'qr' && !isLoggedIn.value && !qrImage.value) await refreshQr()
})

onUnmounted(() => {
    if (pollTimer) clearInterval(pollTimer)
})
</script>

<style scoped>
.login-setting {
    width: 100%;
    min-width: 0;
}

.qr-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
}

.qr-img {
    width: 160px;
    height: 160px;
    /* 深色模式下二维码与页面背景易融合，添加白底和内边距形成白色边框，便于扫描定位 */
    background-color: #fff;
    padding: 8px;
    border-radius: 8px;
    box-sizing: border-box;
}

.qr-tips {
    font-size: 13px;
    color: var(--color-text-secondary);
    text-align: center;
}

.logged-in {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
}

.logged-in > span {
    min-width: 0;
    overflow-wrap: anywhere;
}

.logged-in-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
}

.logged-in :deep(.n-button) {
    flex-shrink: 0;
}
</style>
