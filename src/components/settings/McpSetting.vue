<template>
    <div class="setting-row">
        <div class="setting-label">MCP 接入</div>
        <n-button size="small" @click="open">MCP 接入配置</n-button>
    </div>

    <n-modal v-model:show="show" :mask-closable="false">
        <div class="mcp-card">
            <div class="mcp-header">
                <span class="mcp-title">MCP 接入配置</span>
                <button class="mcp-close" type="button" @click="show = false">关闭</button>
            </div>

            <p class="mcp-desc">通过 MCP 协议，你可以让 AI 助手直接调用此解析器的能力。请注意保护你的密钥。</p>

            <div class="mcp-label">MCP 链接 (包含密钥)</div>
            <div class="mcp-url">
                <textarea ref="urlRef" class="mcp-url-text" readonly rows="2" :value="url"
                    @focus="selectAll"></textarea>
                <n-button size="small" type="primary" class="mcp-copy" :disabled="!url" @click="copy">复制</n-button>
            </div>

            <div class="mcp-actions">
                <n-button type="error" :loading="resetting" @click="confirmReset">重置密钥</n-button>
                <n-button type="primary" @click="show = false">完成</n-button>
            </div>
        </div>
    </n-modal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NModal, useDialog, useNotification } from 'naive-ui'
import { fetchMcpKey, resetMcpKey } from '../../api/webClient'

const notification = useNotification()
const dialog = useDialog()
const show = ref(false)
const url = ref('')
const resetting = ref(false)
const urlRef = ref<HTMLTextAreaElement | null>(null)

function buildUrl(key: string): string {
    return `${window.location.origin}/mcp?key=${key}`
}

function selectAll() {
    urlRef.value?.select()
}

async function open() {
    show.value = true
    try {
        url.value = buildUrl(await fetchMcpKey())
    } catch (error) {
        notification.error({
            title: '读取 MCP 配置失败',
            content: error instanceof Error ? error.message : String(error),
        })
    }
}

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
.mcp-card {
    width: min(460px, 92vw);
    padding: 20px;
    background: var(--surface, #fff);
    border-radius: 16px;
    box-shadow: var(--shadow-lg, 0 8px 32px rgba(0, 0, 0, 0.08));
}

.mcp-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 14px;
}

.mcp-title {
    font-size: 18px;
    font-weight: 700;
    color: var(--accent, #0071e3);
}

.mcp-close {
    border: none;
    background: none;
    color: var(--danger, #ff3b30);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
}

.mcp-desc {
    margin: 0 0 16px;
    padding: 12px 14px;
    background: var(--bg, #f5f5f7);
    border-radius: 10px;
    color: var(--text-secondary, #86868b);
    font-size: 13px;
    line-height: 1.6;
}

.mcp-label {
    margin-bottom: 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--accent, #0071e3);
}

.mcp-url {
    padding: 10px;
    border: 1px solid var(--border, #d2d2d7);
    border-radius: 10px;
}

.mcp-url-text {
    width: 100%;
    border: none;
    outline: none;
    resize: none;
    background: transparent;
    color: var(--text-primary, #1d1d1f);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12.5px;
    line-height: 1.6;
    word-break: break-all;
}

.mcp-copy {
    display: block;
    margin-left: auto;
    margin-top: 8px;
}

.mcp-actions {
    display: flex;
    gap: 12px;
    margin-top: 18px;
}

.mcp-actions :deep(.n-button) {
    flex: 1;
    height: 42px;
}
</style>
