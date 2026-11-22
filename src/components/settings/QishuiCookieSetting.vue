<template>
    <div class="qishui-cookie">
        <n-form-item label="登录 Cookie">
            <n-input v-model:value="draft" type="textarea" :rows="3" :disabled="busy"
                placeholder="sessionid=...; uid_tt=...; sid_guard=..." />
        </n-form-item>
        <n-form-item label="状态">
            <div class="status-line">
                <n-tag :type="status.configured ? 'success' : 'default'" size="small" round>
                    {{ status.configured ? '已配置' : '未配置' }}
                </n-tag>
                <n-text depth="3" class="status-detail">{{ statusText }}</n-text>
            </div>
        </n-form-item>
        <n-form-item label="签名器">
            <div class="status-line">
                <n-tag :type="signerTagType" size="small" round>{{ signerLabel }}</n-tag>
                <n-text depth="3" class="status-detail">{{ signerDetail }}</n-text>
            </div>
        </n-form-item>
        <n-form-item label=" ">
            <n-space>
                <n-button type="primary" :loading="busy" :disabled="!draft.trim()" @click="handleSave">
                    保存 Cookie
                </n-button>
                <n-button v-if="status.configured" :disabled="busy" @click="handleClear">清除</n-button>
            </n-space>
        </n-form-item>
        <n-alert v-if="error" type="error" :show-icon="false" class="feedback">{{ error }}</n-alert>
        <n-alert v-else-if="notice" type="success" :show-icon="false" class="feedback">{{ notice }}</n-alert>
        <n-text depth="3" class="tip">
            获取方式：本机汽水客户端登录后，用脚本一键读取
            （<code>python .workbuddy/skills/qishui-login/scripts/qishui_login.py client --mcp</code>），
            或从客户端的网络请求里复制（至少包含 sessionid）。搜索与高音质还需要签名器可用；
            Cookie 会过期，失效后重新读取即可。
        </n-text>
    </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { NAlert, NButton, NFormItem, NInput, NSpace, NTag, NText } from 'naive-ui'
import {
    clearQishuiCookie,
    getQishuiCookieStatus,
    getQishuiSignerStatus,
    saveQishuiCookie,
    type QishuiCookieStatus,
    type QishuiSignerStatus,
} from '../../api/qishuiApi'

const draft = ref('')
const busy = ref(false)
const error = ref('')
const notice = ref('')
const status = ref<QishuiCookieStatus>({ configured: false, length: 0, keys: '' })
const signer = ref<QishuiSignerStatus | null>(null)

const signerLabel = computed(() => {
    if (!signer.value) return '检测中'
    if (!signer.value.available) return '不可用'
    return signer.value.mode === 'remote' ? '远程签名' : '本机签名'
})

const signerTagType = computed<'default' | 'success' | 'warning'>(() => {
    if (!signer.value) return 'default'
    return signer.value.available ? 'success' : 'warning'
})

const signerDetail = computed(() => {
    const value = signer.value
    if (!value) return '正在检查签名能力'
    if (!value.available) {
        return '未找到本机 bdms.node，也没配置远程签名服务：搜索与高音质不可用，只能解析分享链接'
    }
    if (value.mode === 'remote') {
        return `宿主机签名服务 ${value.signerUrl}（跑在容器外，需保持在线）`
    }
    const bdms = value.bdms.split(/[\\/]/).pop() || 'bdms.node'
    return `本机 ${bdms}${value.electronMode ? ' · 使用客户端自带 Electron' : ''}`
})

const statusText = computed(() => {
    if (!status.value.configured) {
        return '未配置时接口按匿名请求处理，搜索会返回空结果'
    }
    const keys = status.value.keys || '未知'
    return `${status.value.length} 字符 · ${keys}`
})

async function load() {
    try {
        status.value = await getQishuiCookieStatus()
    } catch (e) {
        error.value = e instanceof Error ? e.message : String(e)
    }
    try {
        signer.value = await getQishuiSignerStatus()
    } catch {
        // 老版本服务端没有该接口时按「不可用」显示，不影响 Cookie 操作。
        signer.value = { available: false, mode: '', bdms: '', runtime: '', signerUrl: '', electronMode: false, deviceId: '' }
    }
}

async function handleSave() {
    error.value = ''
    notice.value = ''
    busy.value = true
    try {
        status.value = await saveQishuiCookie(draft.value)
        draft.value = ''
        notice.value = 'Cookie 已保存，立即生效'
    } catch (e) {
        error.value = e instanceof Error ? e.message : String(e)
    } finally {
        busy.value = false
    }
}

async function handleClear() {
    error.value = ''
    notice.value = ''
    busy.value = true
    try {
        await clearQishuiCookie()
        status.value = { configured: false, length: 0, keys: '' }
        notice.value = 'Cookie 已清除，恢复匿名请求'
    } catch (e) {
        error.value = e instanceof Error ? e.message : String(e)
    } finally {
        busy.value = false
    }
}

onMounted(load)
</script>

<style scoped>
.qishui-cookie {
    width: 100%;
}

.status-line {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
}

.status-detail {
    font-size: 12.5px;
}

.feedback {
    margin-bottom: 10px;
}

.tip {
    display: block;
    font-size: 12px;
    line-height: 1.7;
}
</style>
