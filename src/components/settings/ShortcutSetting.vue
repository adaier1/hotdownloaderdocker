<template>
    <n-form-item label="调用地址">
        <div class="shortcut-row">
            <input ref="urlRef" class="set-input mono" type="text" readonly :value="url" @focus="selectAll" />
            <button type="button" class="btn-ghost" :disabled="!url" @click="copy">复制</button>
            <button type="button" class="btn-ghost" :disabled="resetting" @click="confirmReset">重置 Key</button>
        </div>
    </n-form-item>
    <n-form-item label="用法">
        <div class="shortcut-help">
            <p>
                在 iOS 捷径里用「获取 URL 内容」，地址填上面这条，末尾直接拼接分享链接即可。
                链接里的 <code>:</code> <code>/</code> <code>?</code> <code>&amp;</code> 需要转义，
                捷径里用「URL 编码」动作处理一下：
            </p>
            <code class="shortcut-sample">{{ url }}<em>https%3A%2F%2Fy.qq.com%2F...</em></code>
            <p>
                调用后直接创建下载任务，不需要在页面上确认。返回 JSON 里的
                <code>message</code> 可以直接显示到通知里，<code>songs</code> 是已创建的任务列表。
                歌单一次最多处理 20 首（可用 <code>&amp;limit=</code> 调整，上限 100）。
            </p>
        </div>
    </n-form-item>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { NFormItem, useDialog, useNotification } from 'naive-ui'
import { fetchShortcutKey, resetShortcutKey } from '../../api/webClient'
import { copyText } from '../../utils/clipboard'

const notification = useNotification()
const dialog = useDialog()
const url = ref('')
const resetting = ref(false)
const urlRef = ref<HTMLInputElement | null>(null)

function buildUrl(key: string): string {
    return `${window.location.origin}/api/shortcut/parse?key=${key}&url=`
}

function selectAll() {
    urlRef.value?.select()
}

async function loadKey() {
    try {
        url.value = buildUrl(await fetchShortcutKey())
    } catch (error) {
        notification.error({
            title: '读取快捷指令配置失败',
            content: error instanceof Error ? error.message : String(error),
        })
    }
}

onMounted(loadKey)

async function copy() {
    if (!url.value) return
    if (await copyText(url.value)) {
        notification.success({ title: '已复制', content: '调用地址已复制到剪贴板。' })
        return
    }
    selectAll()
    notification.warning({ title: '复制失败', content: '请手动复制已选中的链接。' })
}

function confirmReset() {
    dialog.warning({
        title: '重置 Key',
        content: '重置后旧地址立即失效，需要更新所有捷径与脚本。确定继续吗？',
        positiveText: '重置',
        negativeText: '取消',
        onPositiveClick: async () => {
            resetting.value = true
            try {
                url.value = buildUrl(await resetShortcutKey())
                notification.success({ title: '已重置', content: '新的调用地址已生成。' })
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
.shortcut-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-width: 0;
    flex-wrap: wrap;
}

.shortcut-row .set-input {
    flex: 1 1 480px;
}

.shortcut-help {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 720px;
    font-size: 12.5px;
    line-height: 1.7;
    color: var(--text-secondary);
}

.shortcut-help p {
    margin: 0;
}

.shortcut-help code {
    font-family: monospace;
    font-size: 12px;
    padding: 1px 4px;
    border-radius: 4px;
    background: var(--surface);
}

.shortcut-sample {
    display: block;
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--surface);
    word-break: break-all;
}

.shortcut-sample em {
    font-style: normal;
    color: var(--text-tertiary);
}
</style>
