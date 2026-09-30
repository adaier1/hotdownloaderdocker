<template>
    <div v-if="!loading && keywords.length > 0" class="hot-keywords">
        <div class="hot-header">热搜推荐</div>
        <div class="hot-list">
            <button v-for="word in keywords" :key="word" type="button" class="md-chip" @click="$emit('select', word)">
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
    margin-top: 16px;
}

.hot-header {
    font-size: 14.5px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 12px;
}

.hot-list {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
}

.hot-loading {
    display: flex;
    justify-content: center;
    padding: 12px 0;
}
</style>
