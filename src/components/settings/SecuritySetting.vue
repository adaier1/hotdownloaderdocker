<template>
    <div class="setting-row">
        <div class="setting-label">访问密码</div>
        <n-button size="small" @click="show = true">修改密码</n-button>
    </div>
    <div class="setting-row">
        <div class="setting-label">登录状态</div>
        <n-button size="small" @click="confirmLogout">退出登录</n-button>
    </div>

    <n-modal v-model:show="show" preset="card" title="修改访问密码" :style="{ width: 'min(420px, 92vw)' }">
            <n-form :show-feedback="false">
                <n-form-item label="当前密码" :label-width="90">
                    <n-input v-model:value="currentPassword" type="password" show-password-on="click"
                        placeholder="请输入当前密码" />
                </n-form-item>
                <n-form-item label="新密码" :label-width="90">
                    <n-input v-model:value="newPassword" type="password" show-password-on="click"
                        placeholder="至少 4 个字符" />
                </n-form-item>
                <n-form-item label="确认新密码" :label-width="90">
                    <n-input v-model:value="confirmPassword" type="password" show-password-on="click"
                        placeholder="再次输入新密码" />
                </n-form-item>
            </n-form>
            <template #footer>
                <div class="modal-actions">
                    <n-button @click="close">取消</n-button>
                    <n-button type="primary" :loading="saving" @click="submit">保存</n-button>
                </div>
            </template>
        </n-modal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NButton, NForm, NFormItem, NInput, NModal, useDialog, useNotification } from 'naive-ui'
import { applyWebCredential, changeWebPassword, currentWebUsername, logoutWeb, webSession } from '../../api/webClient'

const notification = useNotification()
const dialog = useDialog()
const show = ref(false)
const saving = ref(false)
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')

function close() {
    show.value = false
    currentPassword.value = ''
    newPassword.value = ''
    confirmPassword.value = ''
}

function confirmLogout() {
    dialog.warning({
        title: '退出登录',
        content: '确定要退出当前登录状态吗？退出后需重新输入密码。',
        positiveText: '退出',
        negativeText: '取消',
        onPositiveClick: () => logoutWeb(),
    })
}

async function submit() {
    if (webSession.mode !== 'password') {
        notification.warning({ title: '无法修改', content: '当前为令牌模式，请在部署环境中修改令牌。' })
        return
    }
    if (newPassword.value.length < 4) {
        notification.warning({ title: '新密码过短', content: '新密码至少需要 4 个字符。' })
        return
    }
    if (newPassword.value !== confirmPassword.value) {
        notification.warning({ title: '两次输入不一致', content: '请确认两次输入的新密码相同。' })
        return
    }
    saving.value = true
    try {
        await changeWebPassword(currentPassword.value, newPassword.value)
        applyWebCredential(currentWebUsername() || 'admin', newPassword.value)
        notification.success({ title: '修改成功', content: '访问密码已更新。' })
        close()
    } catch (error) {
        notification.error({
            title: '修改失败',
            content: error instanceof Error ? error.message : String(error),
        })
    } finally {
        saving.value = false
    }
}
</script>

<style scoped>
.modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
}
</style>
