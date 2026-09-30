<template>
    <div class="md-pills task-tabs">
        <button v-for="tab in TABS" :key="tab.key" type="button" class="md-pill"
            :class="{ active: activeTab === tab.key }" @click="$emit('update:activeTab', tab.key)">
            {{ tab.label }}（{{ countOf(tab.countKey) }}）
        </button>
    </div>
</template>

<script setup lang="ts">
export interface TabCounts {
    total: number
    waiting: number
    downloading: number
    paused: number
    completed: number
    interrupted: number
    error: number
}

const props = defineProps<{
    activeTab: string
    counts: TabCounts
}>()

defineEmits<{
    (e: 'update:activeTab', value: string): void
}>()

const TABS: Array<{ key: string; label: string; countKey: keyof TabCounts }> = [
    { key: 'all', label: '全部', countKey: 'total' },
    { key: 'waiting', label: '等待中', countKey: 'waiting' },
    { key: 'downloading', label: '下载中', countKey: 'downloading' },
    { key: 'paused', label: '暂停', countKey: 'paused' },
    { key: 'completed', label: '已完成', countKey: 'completed' },
    { key: 'interrupted', label: '已中断', countKey: 'interrupted' },
    { key: 'error', label: '错误', countKey: 'error' },
]

function countOf(key: keyof TabCounts): number {
    return props.counts[key] ?? 0
}
</script>

<style scoped>
.task-tabs {
    margin-bottom: 16px;
}

@media (max-width: 767px) {
    .md-pill {
        min-height: 44px;
    }
}
</style>
