<template>
    <div class="search-history" v-if="history.length > 0">
        <div class="history-header">
            <span class="section-label">搜索历史</span>
            <button type="button" class="link-danger" @click="$emit('clear')">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                    <path d="M3 6h18M8 6V4h8v2m1 0-1 14H8L7 6" />
                </svg>
                清除搜索历史
            </button>
        </div>
        <div class="history-chips">
            <span v-for="item in history" :key="item" class="md-chip">
                <button type="button" class="chip-text" @click="$emit('select', item)">{{ item }}</button>
                <button type="button" class="chip-remove" aria-label="删除" @click="$emit('remove', item)">×</button>
            </span>
        </div>
    </div>
</template>

<script setup lang="ts">
defineProps<{
    history: string[]
}>()

defineEmits<{
    (e: 'select', keyword: string): void
    (e: 'remove', keyword: string): void
    (e: 'clear'): void
}>()
</script>

<style scoped>
.search-history {
    margin-bottom: 16px;
}

.history-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
}

.section-label {
    font-size: 14.5px;
    font-weight: 600;
    color: var(--text-primary);
}

.link-danger {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: none;
    border: none;
    color: var(--danger);
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    padding: 4px 8px;
    border-radius: 6px;
    cursor: pointer;
    transition: background var(--transition);
}

.link-danger:hover {
    background: var(--danger-weak);
}

.link-danger svg {
    width: 13px;
    height: 13px;
}

.history-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
}

.md-chip {
    padding: 0 6px 0 0;
    gap: 2px;
}

.chip-text {
    border: none;
    background: none;
    font: inherit;
    color: inherit;
    cursor: pointer;
    padding: 8px 6px 8px 18px;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.chip-remove {
    border: none;
    background: none;
    color: var(--text-tertiary);
    cursor: pointer;
    font-size: 15px;
    line-height: 1;
    padding: 6px 10px;
    border-radius: 999px;
}

.chip-remove:hover {
    color: var(--danger);
    background: var(--danger-weak);
}
</style>
