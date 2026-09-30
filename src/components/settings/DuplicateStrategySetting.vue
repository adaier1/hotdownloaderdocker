<template>
    <n-form-item label="覆盖策略">
        <n-radio-group :value="settingsStore.settings.duplicateStrategy"
            @update:value="(val) => (settingsStore.settings.duplicateStrategy = val)">
            <n-space :size="24">
                <n-radio value="ask">询问</n-radio>
                <n-radio value="overwrite">覆盖</n-radio>
                <n-radio value="rename">保留两份</n-radio>
                <n-radio value="cancel">取消</n-radio>
            </n-space>
        </n-radio-group>
    </n-form-item>
</template>

<script setup lang="ts">
import { NFormItem, NRadioGroup, NRadio, NSpace } from 'naive-ui'
import { useSettingsStore } from '../../stores/settingsStore'

const settingsStore = useSettingsStore()
</script>
