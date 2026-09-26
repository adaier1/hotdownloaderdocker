<template>
    <div class="task-tabs" role="tablist">
        <button v-for="tab in tabs" :key="tab.key" type="button" class="task-tab"
            :class="{ active: activeTab === tab.key }" role="tab" :aria-selected="activeTab === tab.key"
            @click="$emit('update:activeTab', tab.key)">
            {{ tab.label }} ({{ counts[tab.countKey] }})
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
    error: number
}

defineProps<{
    activeTab: string
    counts: TabCounts
}>()

defineEmits<{
    (e: 'update:activeTab', value: string): void
}>()

const tabs: Array<{ key: string; label: string; countKey: keyof TabCounts }> = [
    { key: 'all', label: '全部', countKey: 'total' },
    { key: 'waiting', label: '等待中', countKey: 'waiting' },
    { key: 'downloading', label: '下载中', countKey: 'downloading' },
    { key: 'paused', label: '暂停', countKey: 'paused' },
    { key: 'completed', label: '已完成', countKey: 'completed' },
    { key: 'error', label: '错误', countKey: 'error' },
]
</script>

<style scoped>
.task-tabs {
    display: flex;
    gap: 2px;
    background: var(--surface);
    border-radius: var(--radius-md);
    padding: 3px;
    margin-bottom: 20px;
    box-shadow: var(--shadow-sm);
    width: fit-content;
    max-width: 100%;
    overflow-x: auto;
}

.task-tab {
    padding: 7px 16px;
    border-radius: 9px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
    border: none;
    background: none;
    font-family: inherit;
    white-space: nowrap;
    flex-shrink: 0;
}

.task-tab:hover {
    color: var(--text-primary);
}

.task-tab.active {
    background: var(--bg);
    color: var(--text-primary);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
}
</style>
