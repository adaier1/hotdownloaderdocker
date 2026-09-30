<template>
    <n-form-item label="搜索历史">
        <button type="button" class="btn-ghost" @click="historyStore.clearHistory()">清除搜索历史</button>
    </n-form-item>
</template>

<script setup lang="ts">
import { NFormItem } from 'naive-ui'
import { useHistoryStore } from '../../stores/historyStore'

const historyStore = useHistoryStore()
</script>
