<template>
    <n-form-item label="文件名模板">
        <div class="template-field">
            <n-input :value="settingsStore.settings.namingTemplate"
                @update:value="(val) => (settingsStore.settings.namingTemplate = val)" placeholder="{song} - {artist}" />
            <div class="template-help">
                可使用变量：<code>{song}</code>（歌名）、<code>{artist}</code>（歌手）、<code>{album}</code>（专辑）、<code>{quality}</code>（音质）<br />
                若替换后结果为空或仅含非法字符，将自动使用默认模板“歌名 - 歌手”
            </div>
        </div>
    </n-form-item>
</template>

<script setup lang="ts">
import { NFormItem, NInput } from 'naive-ui'
import { useSettingsStore } from '../../stores/settingsStore'

const settingsStore = useSettingsStore()
</script>

<style scoped>
.template-field {
    width: 100%;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.template-help {
    font-size: 12px;
    color: var(--color-text-secondary);
    line-height: 1.6;
}

code {
    background: rgba(127, 127, 127, 0.15);
    padding: 1px 4px;
    border-radius: 3px;
    font-family: monospace;
    font-size: 12px;
    color: inherit;
}
</style>
