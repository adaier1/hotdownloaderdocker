<template>
    <n-form-item label="歌手分隔符">
        <div class="separator-field">
            <n-input style="width: 90px" :value="settingsStore.settings.artistSeparator"
                @update:value="(val) => (settingsStore.settings.artistSeparator = val)" placeholder="、" />
            <div class="separator-help">
                用于拼接多歌手场景下的分隔字符串，可填入 <code>&amp;</code>、<code>/</code>、<code>，</code>等任意文本。<br />
                留空时将回退为默认的中文顿号：<code>、</code>。
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
.separator-field {
    width: 100%;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.separator-help {
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
