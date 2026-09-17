<template>
    <n-form-item label="下载目录">
        <n-input-group>
            <n-input :value="settingsStore.settings.downloadDir" readonly placeholder="服务端下载目录" />
            <n-button type="primary" @click="selectDirectory">修改</n-button>
        </n-input-group>
    </n-form-item>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { NFormItem, NInput, NInputGroup, NButton } from 'naive-ui'
import { useSettingsStore } from '../../stores/settingsStore'

const settingsStore = useSettingsStore()

// Web 版运行在服务端容器中，下载目录由 DOWNLOAD_DIR 卷决定。
// 浏览器无法直接选择服务端目录，这里允许输入容器内的绝对路径。
onMounted(() => {
    if (!settingsStore.settings.downloadDir) {
        settingsStore.getDefaultDownloadDir()
    }
})

function selectDirectory() {
    const current = settingsStore.settings.downloadDir || ''
    const selected = window.prompt('请输入服务端下载目录（容器内绝对路径）', current)
    if (selected && selected.trim()) {
        settingsStore.settings.downloadDir = selected.trim()
    }
}
</script>
