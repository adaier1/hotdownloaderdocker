<template>
    <n-form-item label="接入地址">
        <div class="mcp-row">
            <input ref="urlRef" class="set-input mono" type="text" readonly :value="url" @focus="selectAll" />
            <button type="button" class="btn-ghost" :disabled="!url" @click="copy">复制</button>
            <button type="button" class="btn-ghost" :disabled="resetting" @click="confirmReset">重置</button>
        </div>
    </n-form-item>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { NFormItem, useDialog, useNotification } from 'naive-ui'
import { fetchMcpKey, resetMcpKey } from '../../api/webClient'

const notification = useNotification()
const dialog = useDialog()
const url = ref('')
const resetting = ref(false)
const urlRef = ref<HTMLInputElement | null>(null)

function buildUrl(key: string): string {
    return `${window.location.origin}/mcp?key=${key}`
}

function selectAll() {
    urlRef.value?.select()
}

async function loadKey() {
    try {
        url.value = buildUrl(await fetchMcpKey())
    } catch (error) {
        notification.error({
            title: '读取 MCP 配置失败',
            content: error instanceof Error ? error.message : String(error),
        })
    }
}

onMounted(loadKey)

async function copy() {
    if (!url.value) return
    try {
        await navigator.clipboard.writeText(url.value)
        notification.success({ title: '已复制', content: 'MCP 链接已复制到剪贴板。' })
    } catch {
        // 剪贴板不可用时回退到选中文本，方便手动复制。
        selectAll()
        notification.warning({ title: '复制失败', content: '请手动复制已选中的链接。' })
    }
}

function confirmReset() {
    dialog.warning({
        title: '重置密钥',
        content: '重置后旧链接会立即失效，需要更新所有 MCP 客户端配置。确定继续吗？',
        positiveText: '重置',
        negativeText: '取消',
        onPositiveClick: async () => {
            resetting.value = true
            try {
                url.value = buildUrl(await resetMcpKey())
                notification.success({ title: '已重置', content: '新的 MCP 链接已生成。' })
            } catch (error) {
                notification.error({
                    title: '重置失败',
                    content: error instanceof Error ? error.message : String(error),
                })
            } finally {
                resetting.value = false
            }
        },
    })
}
</script>

<style scoped>
.mcp-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-width: 0;
    flex-wrap: wrap;
}

/* 给输入框一个较大的基准宽度，空间不足时按钮换行，保证完整显示链接 */
.mcp-row .set-input {
    flex: 1 1 480px;
}
</style>
