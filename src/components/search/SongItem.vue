<template>
    <div class="song-item" :class="{ 'is-selected': selected }">
        <n-checkbox :checked="selected" @update:checked="$emit('toggleSelect', $event)" />

        <div class="cover">
            <img v-if="coverUrl" :src="coverUrl" class="cover-img" alt="封面" loading="lazy" />
            <div v-else class="cover-img cover-fallback" v-html="NOTE_SVG" />
        </div>

        <div class="info">
            <div class="title">
                {{ song.title }}
                <span v-if="song.preview" class="preview-tag" title="音源只返回试听片段">试听</span>
            </div>
            <div class="artist">
                <ArtistNames :platform="song.platform" :artists="song.artists" :fallback="song.artist"
                    @click-artist="(platform, artist) => $emit('click-artist', platform, artist)" />
            </div>
            <div v-if="song.album" class="album">
                <n-button v-if="albumId" text size="small" class="album-link" @click.stop="$emit('click-album', song)">
                    {{ song.album }}
                </n-button>
                <span v-else>{{ song.album }}</span>
            </div>
        </div>

        <div class="quality-tags">
            <span v-for="q in sortedQualities.slice(0, 4)" :key="q.quality" class="md-tag"
                :class="{ gold: isGoldQuality(q.quality) }">
                {{ q.quality }}
            </span>
            <span v-if="sortedQualities.length > 4" class="md-tag gray">+{{ sortedQualities.length - 4 }}</span>
        </div>

        <button type="button" class="md-btn-dl" @click="$emit('download', song)">下载</button>
    </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { NCheckbox, NButton } from 'naive-ui'
import type { ArtistReference, SongInfo } from '../../types'
import { ALL_QUALITY_ORDER } from '../../types'
import { fetchCover } from '../../api/musicApi'
import ArtistNames from './ArtistNames.vue'
import { getMusicEntityId } from '../../utils/music'

const NOTE_SVG = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/></svg>'

// 高音质使用金色标签，与 MusicDock 的臻品母带标记一致。
const GOLD_QUALITIES = new Set(['flac', 'ape', 'hires'])
function isGoldQuality(quality: string): boolean {
    return GOLD_QUALITIES.has(quality) || quality.includes('臻品')
}

const props = defineProps<{
    song: SongInfo
    selected: boolean
}>()

defineEmits<{
    (e: 'toggleSelect', selected: boolean): void
    (e: 'download', song: SongInfo): void
    (e: 'click-artist', platform: string, artist: ArtistReference): void
    (e: 'click-album', song: SongInfo): void
}>()

const albumId = computed(() => getMusicEntityId(props.song.platform, props.song.albumId, props.song.albumMid))

// 按品质从高到低排序
const sortedQualities = computed(() => {
    return [...props.song.qualities].sort((a, b) => {
        const ia = ALL_QUALITY_ORDER.indexOf(a.quality)
        const ib = ALL_QUALITY_ORDER.indexOf(b.quality)
        const idxA = ia === -1 ? -1 : ia
        const idxB = ib === -1 ? -1 : ib
        return idxB - idxA
    })
})

const coverUrl = ref<string>('')

async function loadCoverIfNeeded() {
    if (props.song.coverUrl) {
        coverUrl.value = props.song.coverUrl
        return
    }
    if (!props.song.id) return
    try {
        const url = await fetchCover('kuwo', props.song.id)
        if (props.song.id === props.song.id) {
            coverUrl.value = url
        }
    } catch {
        // 加载失败保持占位
    }
}

onMounted(() => {
    loadCoverIfNeeded()
})

watch(() => props.song.id, () => {
    coverUrl.value = props.song.coverUrl
    loadCoverIfNeeded()
})
</script>

<style scoped>
.song-item {
    display: flex;
    align-items: center;
    gap: 16px;
    min-width: 0;
    padding: 13px 16px 13px 18px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: none;
    transition: border-color var(--transition), box-shadow var(--transition), background var(--transition);
}

.song-item:hover {
    border-color: var(--border-2);
    background: #fffefb;
}

.song-item.is-selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
}

.cover {
    width: 46px;
    height: 46px;
    flex-shrink: 0;
}

.cover-img {
    width: 46px;
    height: 46px;
    border-radius: 10px;
    object-fit: cover;
    display: block;
}

.cover-fallback {
    display: grid;
    place-items: center;
    color: var(--text-tertiary);
    background: var(--surface-sunken);
    border: 1px solid var(--border-light);
}

.cover-fallback :deep(svg) {
    width: 20px;
    height: 20px;
    opacity: 0.9;
}

.info {
    flex: 1;
    min-width: 0;
    overflow: hidden;
}

.title {
    font-size: 13.5px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-primary);
    line-height: 1.5;
}

.preview-tag {
    margin-left: 6px;
    padding: 0 5px;
    font-size: 10.5px;
    font-weight: 400;
    line-height: 15px;
    color: var(--text-tertiary);
    border: 1px solid var(--border);
    border-radius: 4px;
    vertical-align: 1px;
}

.artist {
    margin-top: 3px;
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.album {
    margin-top: 1px;
    font-size: 11.5px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.album-link {
    font: inherit;
    vertical-align: baseline;
}

.quality-tags {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
    max-width: 360px;
    flex-shrink: 0;
}

/* 手机保留封面和操作入口，长歌名在剩余空间内省略 */
@media (max-width: 767px) {
    .song-item {
        gap: 12px;
        padding: 14px;
    }

    .cover,
    .cover-img {
        width: 48px;
        height: 48px;
    }

    .quality-tags {
        display: none;
    }

    .md-btn-dl {
        min-height: 40px;
        padding: 0 16px;
    }
}
</style>
