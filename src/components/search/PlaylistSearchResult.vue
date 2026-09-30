<template>
    <div class="playlist-search-result">
        <template v-if="playlists.length > 0">
            <div class="playlist-card-list">
                <div v-for="pl in playlists" :key="pl.id" class="playlist-card" @click="$emit('click-playlist', pl)">
                    <img v-if="pl.coverUrl" :src="pl.coverUrl" class="playlist-card-cover" alt="歌单封面" />
                    <div class="playlist-card-info">
                        <div class="playlist-card-name">{{ pl.name }}</div>
                        <div class="playlist-card-creator">{{ pl.creator }}</div>
                        <div class="playlist-card-meta">{{ pl.songCount }} 首 · {{ formatPlayCount(pl.playCount) }}</div>
                    </div>
                </div>
            </div>
            <LoadMoreButton v-if="hasMore" :loading="loadingMore" :disabled="loadingMore" @click="$emit('load-more')" />
        </template>
        <div v-else class="empty-result">
            <n-empty description="未找到相关歌单" />
        </div>
    </div>
</template>

<script setup lang="ts">
import { NEmpty } from 'naive-ui'
import type { PlaylistSearchItem } from '../../types'
import LoadMoreButton from './LoadMoreButton.vue'
import { formatPlayCount } from '../../utils/format'

defineProps<{
    playlists: PlaylistSearchItem[]
    hasMore: boolean
    loadingMore: boolean
}>()

defineEmits<{
    (e: 'click-playlist', playlist: PlaylistSearchItem): void
    (e: 'load-more'): void
}>()
</script>

<style scoped>
.playlist-card-list {
    display: flex;
    flex-direction: column;
    gap: 14px;
}

.playlist-card {
    display: flex;
    gap: 16px;
    align-items: center;
    min-width: 0;
    padding: 16px 20px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
    cursor: pointer;
    transition: border-color var(--transition);
}

.playlist-card:hover {
    border-color: var(--accent);
}

.playlist-card-cover {
    width: 56px;
    height: 56px;
    border-radius: 9px;
    object-fit: cover;
    flex-shrink: 0;
}

.playlist-card-info {
    flex: 1;
    min-width: 0;
}

.playlist-card-name {
    font-size: 15.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-primary);
}

.playlist-card-creator {
    overflow-wrap: anywhere;
    color: var(--text-secondary);
    font-size: 13px;
    margin-top: 4px;
}

.playlist-card-meta {
    overflow-wrap: anywhere;
    color: var(--text-tertiary);
    font-size: 12.5px;
    margin-top: 3px;
}

.empty-result {
    display: flex;
    justify-content: center;
    padding: 40px 0;
}
</style>
