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
            获取方式：登录汽水音乐网页版后，在浏览器开发者工具的网络请求里复制 Cookie（至少包含
            sessionid）。保存后搜索、详情与歌词都会带上登录态；Cookie 会过期，失效后重新粘贴即可。
        </n-text>
    </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { NAlert, NButton, NFormItem, NInput, NSpace, NTag, NText } from 'naive-ui'
import {
    clearQishuiCookie,
    getQishuiCookieStatus,
    saveQishuiCookie,
    type QishuiCookieStatus,
} from '../../api/qishuiApi'

const draft = ref('')
const busy = ref(false)
const error = ref('')
const notice = ref('')
const status = ref<QishuiCookieStatus>({ configured: false, length: 0, keys: '' })

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
