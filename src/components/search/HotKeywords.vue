<template>
    <div v-if="!loading && keywords.length > 0" class="hot-keywords">
        <div class="hot-header">热搜推荐</div>
        <div class="hot-list">
            <button v-for="word in keywords" :key="word" type="button" class="tag" @click="$emit('select', word)">
                {{ word }}
            </button>
        </div>
    </div>
    <div v-else-if="loading" class="hot-loading">
        <n-spin size="small" />
    </div>
</template>

<script setup lang="ts">
import { NSpin } from 'naive-ui'

defineProps<{
    keywords: string[]
    loading: boolean
}>()

defineEmits<{
    (e: 'select', word: string): void
}>()
</script>

<style scoped>
.hot-keywords {
    margin-bottom: 28px;
}

.hot-header {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 12px;
}

.hot-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
}

.tag {
    display: inline-flex;
    align-items: center;
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

.hot-loading {
    display: flex;
    justify-content: center;
    padding: 12px 0;
}
</style>
