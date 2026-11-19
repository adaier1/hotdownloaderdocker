<template>
    <div class="segmented" role="tablist">
        <button v-for="tab in TABS" :key="tab.key" type="button" role="tab" class="seg"
            :class="{ active: activeTab === tab.key }" :aria-selected="activeTab === tab.key"
            @click="$emit('update:activeTab', tab.key)">
            <span class="seg-label">{{ tab.label }}</span>
            <span class="seg-count">{{ countOf(tab.countKey) }}</span>
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
/* 分段控件：一枚灰色凹槽 + 一枚黑色滑块，是全站"选择"的统一语言 */
.segmented {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px;
    margin-bottom: 16px;
    width: fit-content;
    max-width: 100%;
    overflow-x: auto;
    scrollbar-width: none;
    background: var(--surface-sunken);
    border: 1px solid var(--border-light);
    border-radius: 999px;
}

.segmented::-webkit-scrollbar {
    display: none;
}

.seg {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 14px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: transparent;
    color: var(--text-secondary);
    font-family: inherit;
    font-size: 12.5px;
    font-weight: 450;
    cursor: pointer;
    white-space: nowrap;
    transition: color var(--transition), background var(--transition), box-shadow var(--transition);
}

.seg:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.7);
}

.seg-count {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
    transition: color var(--transition);
}

.seg.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast);
    font-weight: 500;
    box-shadow: 0 1px 2px rgba(28, 28, 22, 0.2);
}

.seg.active .seg-count {
    color: rgba(255, 255, 255, 0.6);
}

@media (max-width: 767px) {
    .seg {
        height: 36px;
    }
}
</style>
