<template>
    <div class="login-setting">
        <!-- 未登录状态 -->
        <template v-if="!isLoggedIn">
            <n-tabs v-model:value="activeTab" type="line" size="small">
                <n-tab-pane name="qr" tab="扫码登录">
                    <div class="qr-container">
                        <img v-if="qrImage" :src="qrImage" alt="二维码" class="qr-img" />
                        <div class="qr-tips">
                            <span v-if="qrStatus === 'waiting'">请使用酷狗音乐 App 扫码</span>
                            <span v-else-if="qrStatus === 'scanned'">已扫码，请在手机上确认</span>
                            <span v-else-if="qrStatus === 'expired'">二维码已过期，请刷新</span>
                            <span v-else-if="qrStatus === 'error'">登录出错，请重试</span>
                        </div>
                        <n-button size="small" @click="refreshQr" :loading="qrLoading">刷新二维码</n-button>
                    </div>
                </n-tab-pane>
                <n-tab-pane name="manual" tab="手动登录">
                    <n-form :label-placement="isNarrow ? 'top' : 'left'" :label-width="isNarrow ? undefined : 110">
                        <n-form-item label="userid">
                            <n-input v-model:value="manualUserid" placeholder="酷狗标准版 userid（必填）" />
                        </n-form-item>
                        <n-form-item label="token">
                            <n-input v-model:value="manualToken" placeholder="酷狗登录 token（必填）" />
                        </n-form-item>
                        <n-form-item label="MID">
                            <n-input v-model:value="manualMid" placeholder="选填，登录设备 MID" />
                        </n-form-item>
                        <n-form-item label="DFID">
                            <n-input v-model:value="manualDfid" placeholder="选填，登录设备 DFID" />
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
                <span>已登录：{{ userId }}</span>
                <n-button size="small" type="error" @click="handleLogout">退出登录</n-button>
            </div>
        </template>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { NTabs, NTabPane, NForm, NFormItem, NInput, NButton, useNotification } from 'naive-ui'
import { kugouLogin, type KugouCredentials } from '../../api/kugouLogin'
import { useNarrowLayout } from '../../composables/useNarrowLayout'

const notification = useNotification()
const isNarrow = useNarrowLayout()
const isLoggedIn = ref(false)
const userId = ref('')
const activeTab = ref('qr')

// 二维码相关
const qrImage = ref('')
const qrId = ref('')
const qrStatus = ref('waiting')
const qrLoading = ref(false)
let pollTimer: ReturnType<typeof setInterval> | null = null

// 手动登录
const manualUserid = ref('')
const manualToken = ref('')
const manualMid = ref('')
const manualDfid = ref('')
const manualLoading = ref(false)

async function refreshQr() {
    if (pollTimer) clearInterval(pollTimer)
    qrLoading.value = true
    try {
        const res = await kugouLogin('qr')
        qrImage.value = res.qr_image ?? ''
        qrId.value = res.qrcode_id ?? ''
        qrStatus.value = 'waiting'
        startPolling()
    } catch (error) {
        console.error('获取酷狗二维码失败:', error)
        qrStatus.value = 'error'
    } finally {
        qrLoading.value = false
    }
}

function startPolling() {
    if (pollTimer) clearInterval(pollTimer)
    pollTimer = setInterval(async () => {
        try {
            const result = await kugouLogin('check', qrId.value)
            qrStatus.value = result.status ?? 'error'
            if (result.status === 'confirmed') {
                if (pollTimer) clearInterval(pollTimer)
                isLoggedIn.value = true
                userId.value = result.userid ?? ''
            } else if (result.status === 'expired') {
                if (pollTimer) clearInterval(pollTimer)
            }
        } catch (error) {
            console.error('轮询酷狗登录状态失败:', error)
        }
    }, 2000)
}

async function handleManualLogin() {
    if (!manualUserid.value || !manualToken.value) {
        notification.error({ content: 'userid 和 token 为必填项', duration: 3000 })
        return
    }
    manualLoading.value = true
    try {
        const credentials: KugouCredentials = {
            userid: manualUserid.value,
            token: manualToken.value,
            mid: manualMid.value,
            dfid: manualDfid.value,
        }
        const result = await kugouLogin('manual', '', credentials)
        isLoggedIn.value = true
        userId.value = result.userid ?? manualUserid.value
    } catch (error) {
        console.error('酷狗手动登录失败:', error)
        notification.error({ content: '登录失败，请检查 userid 和 token', duration: 3000 })
    } finally {
        manualLoading.value = false
    }
}

async function handleLogout() {
    try {
        await kugouLogin('logout')
        isLoggedIn.value = false
        userId.value = ''
        if (pollTimer) clearInterval(pollTimer)
    } catch (error) {
        console.error('退出酷狗登录失败:', error)
    }
}

onMounted(async () => {
    try {
        const status = await kugouLogin('status')
        isLoggedIn.value = status.logged_in ?? false
        userId.value = status.userid ?? ''
    } catch (error) {
        console.error('获取酷狗登录状态失败:', error)
    }
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

.logged-in :deep(.n-button) {
    flex-shrink: 0;
}
</style>
