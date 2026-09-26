<template>
    <div class="search-history" v-if="history.length > 0">
        <div class="history-header">
            <span class="history-title">搜索历史</span>
            <button type="button" class="history-clear" @click="$emit('clear')">清除历史</button>
        </div>
        <div class="history-tags">
            <button v-for="item in history" :key="item" type="button" class="tag" @click="$emit('select', item)">
                {{ item }}
                <span class="tag-close" @click.stop="$emit('remove', item)">×</span>
            </button>
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
    margin-bottom: 28px;
}

.history-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
}

.history-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
}

.history-clear {
    font-size: 12px;
    color: var(--accent);
    cursor: pointer;
    background: none;
    border: none;
    font-family: inherit;
    transition: opacity var(--transition);
}

.history-clear:hover {
    opacity: 0.7;
}

.history-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
}

.tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    padding: 6px 14px;
    background: var(--surface);
    border-radius: 16px;
    font-size: 12px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
    border: none;
    font-family: inherit;
    box-shadow: var(--shadow-sm);
    overflow-wrap: anywhere;
    text-align: left;
}

.tag:hover {
    color: var(--accent);
    background: var(--accent-light);
}

.tag-close {
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    line-height: 1;
    color: var(--text-tertiary);
    transition: color var(--transition);
}

.tag:hover .tag-close {
    color: var(--text-secondary);
}
</style>
