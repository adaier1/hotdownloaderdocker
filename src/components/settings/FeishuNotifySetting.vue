<template>
    <n-form-item label="飞书通知">
        <n-switch v-model:value="enabled" />
    </n-form-item>

    <n-form-item label="FSKEY">
        <div class="fskey-row">
            <input v-model="fsKey" class="set-input mono"
                placeholder="飞书自定义机器人 FSKEY（也可直接粘贴完整 Webhook 地址）" />
            <button type="button" class="btn-ghost" :disabled="saving" @click="save">保存</button>
            <button type="button" class="btn-ghost" :disabled="testing" @click="test">测试</button>
        </div>
    </n-form-item>

    <p class="set-hint">
        任务完成/失败时将推送到飞书群。FSKEY 获取：飞书群 → 设置 → 群机器人 → 添加「自定义机器人」，
        复制 Webhook 地址中 <code>hook/</code> 之后的部分即为 FSKEY
    </p>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { NFormItem, NSwitch, useNotification } from 'naive-ui'
import { fetchNotifyConfig, saveNotifyConfig, testNotify } from '../../api/webClient'

const notification = useNotification()
const enabled = ref(false)
const fsKey = ref('')
const saving = ref(false)
const testing = ref(false)

function describe(error: unknown): string {
    return error instanceof Error ? error.message : String(error)
}

onMounted(async () => {
    try {
        const config = await fetchNotifyConfig()
        enabled.value = config.enabled
        fsKey.value = config.fsKey
    } catch (error) {
        notification.error({ title: '读取通知配置失败', content: describe(error) })
    }
})

async function save() {
    saving.value = true
    try {
        const saved = await saveNotifyConfig({ enabled: enabled.value, fsKey: fsKey.value.trim() })
        enabled.value = saved.enabled
        fsKey.value = saved.fsKey
        notification.success({ title: '已保存', content: '通知设置已更新。' })
    } catch (error) {
        notification.error({ title: '保存失败', content: describe(error) })
    } finally {
        saving.value = false
    }
}

async function test() {
    testing.value = true
    try {
        // 先落盘再测试，确保测试使用当前填写的 FSKEY。
        await saveNotifyConfig({ enabled: enabled.value, fsKey: fsKey.value.trim() })
        await testNotify()
        notification.success({ title: '发送成功', content: '测试消息已发送到飞书。' })
    } catch (error) {
        notification.error({ title: '发送失败', content: describe(error) })
    } finally {
        testing.value = false
    }
}
</script>

<style scoped>
.fskey-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-width: 0;
    flex-wrap: wrap;
}

/* 给输入框较大的基准宽度，保证 FSKEY 完整显示，空间不足时按钮换行 */
.fskey-row .set-input {
    flex: 1 1 420px;
}

.set-hint code {
    padding: 0 4px;
    background: var(--border-light);
    border-radius: 4px;
    font-family: var(--mono);
}
</style>
