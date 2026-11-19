<template>
    <div class="parse-view">
        <header class="page-head rise" style="--i: 0">
            <div class="page-head-main">
                <div class="eyebrow">Parse · 解析</div>
                <div class="page-title">解析链接</div>
                <div class="page-sub">支持网易云、QQ 音乐、酷狗、酷我的歌单与专辑。链接或整段分享文案都可以。</div>
            </div>
        </header>

        <section class="parse-card rise" style="--i: 1">
            <div class="parse-head">
                <span class="parse-label">SHARE LINK · 分享链接</span>
            </div>

            <n-input v-model:value="input" type="textarea" :autosize="{ minRows: 3, maxRows: 6 }" :disabled="loading"
                placeholder="把 App 里「分享 → 复制链接」的内容整段粘贴到这里" />

            <div v-if="hint" class="parse-hint">
                <span class="hint-dot" />
                识别到 {{ platformName(hint.platform) }} · {{ kindName(hint.kind) }}
            </div>
            <div v-else-if="hasInput" class="parse-hint is-muted">未识别到支持的链接，可点击下方示例查看格式</div>

            <div class="parse-actions">
                <label class="fallback">
                    <span class="fallback-label">只有歌单 ID 时</span>
                    <n-select v-model:value="fallbackPlatform" :options="platformOptions" size="small"
                        class="fallback-select" />
                </label>
                <div class="parse-buttons">
                    <n-button quaternary :disabled="!hasInput || loading" @click="reset">清空</n-button>
                    <n-button type="primary" :loading="loading" :disabled="!hasInput" @click="handleParse">
                        解析
                    </n-button>
                </div>
            </div>

            <div class="examples">
                <span class="examples-label">示例</span>
                <button v-for="item in EXAMPLES" :key="item.url" type="button" class="example-chip"
                    @click="fill(item.url)">
                    {{ item.label }}
                </button>
            </div>
        </section>

        <div v-if="loading" class="state-wrapper">
            <n-spin size="medium" />
        </div>

        <div v-else-if="errorMsg" class="state-wrapper">
            <n-alert type="error" :title="errorMsg" />
        </div>

        <template v-else-if="summary">
            <div class="result-info">
                <img v-if="summary.cover" :src="summary.cover" class="result-cover" alt="封面" />
                <div v-else class="result-cover is-empty" />
                <div class="result-detail">
                    <div class="result-badges">
                        <span class="badge is-accent">{{ platformName(result!.platform) }}</span>
                        <span class="badge">{{ kindName(result!.kind) }}</span>
                    </div>
                    <div class="result-title">{{ summary.title }}</div>
                    <div v-if="summary.subtitle" class="result-sub">{{ summary.subtitle }}</div>
                    <div class="result-meta">{{ summary.meta }}</div>
                    <div v-if="loginNotice" class="parse-notice">
                        <span>{{ loginNotice }}</span>
                        <router-link to="/account" class="parse-notice-link">去账号设置登录</router-link>
                    </div>
                </div>
                <div v-if="singleSong" class="result-actions">
                    <n-button type="primary" :disabled="!singleSong.qualities?.length"
                        @click="downloadSingle(singleSong)">
                        {{ singleSong.qualities?.length ? '下载这首' : '暂无可用音质' }}
                    </n-button>
                </div>
            </div>

            <template v-if="!singleSong">
                <div class="list-header">
                    <n-checkbox :checked="isAllSelected" :indeterminate="isIndeterminate" @update:checked="toggleAll">
                        全选
                    </n-checkbox>
                    <span class="count-text">已选 {{ selectedIds.length }} / {{ songs.length }} 首</span>
                </div>

                <div class="song-items">
                    <SongItem v-for="song in songs" :key="song.mid" :song="song"
                        :selected="selectedIds.includes(song.mid)"
                        @toggle-select="(val) => toggleSelect(song.mid, val)"
                        @download="(song) => downloadSingle(song)" @click-artist="openRelatedArtist"
                        @click-album="openSongAlbum" />
                </div>

                <BatchDownloadBar v-if="selectedIds.length > 0" :selectedCount="selectedIds.length"
                    @batch-download="onBatchDownload" />
            </template>
        </template>

        <div v-else class="state-wrapper">
            <n-empty description="粘贴分享链接后点击解析，即可批量下载" />
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onActivated, watch } from 'vue'
import { useRoute } from 'vue-router'
import { NAlert, NButton, NCheckbox, NEmpty, NInput, NSelect, NSpin } from 'naive-ui'
import SongItem from '../components/search/SongItem.vue'
import BatchDownloadBar from '../components/search/BatchDownloadBar.vue'
import { getLoginStatus, parseShareLink } from '../api/musicApi'
import type { AlbumSongsResponse, PlaylistSongsResponse, ShareLinkParseResponse, SongDetailResponse, SongInfo } from '../types'
import { useDownloadActions } from '../composables/useDownloadActions'
import { useMusicNavigation } from '../composables/useMusicNavigation'
import { useLogStore } from '../stores/logStore'
import { PLATFORMS, DEFAULT_PLATFORM } from '../config/platforms'
import { detectShareHint, PLATFORM_NAMES, LINK_KIND_NAMES } from '../utils/shareLink'
import { formatPlayCount } from '../utils/format'

// 示例只列出确定长期有效的公开歌单，避免示例失效被误判为功能故障。
const EXAMPLES = [
    { label: '网易云歌单链接', url: 'https://music.163.com/#/playlist?id=3778678' },
]

const route = useRoute()
const input = ref('')
const loading = ref(false)
const errorMsg = ref('')
const result = ref<ShareLinkParseResponse | null>(null)
const songs = ref<SongInfo[]>([])
const selectedIds = ref<string[]>([])
const fallbackPlatform = ref(DEFAULT_PLATFORM)
// 解析到哪个平台就只用哪个平台，不跨平台替换；这里只放该平台的登录提醒。
const loginNotice = ref('')

const platformOptions = PLATFORMS.map((item) => ({
    label: PLATFORM_NAMES[item.key] ?? item.label,
    value: item.key,
}))

const hasInput = computed(() => input.value.trim().length > 0)
// 前端只做即时提示，最终平台与类型仍以服务端解析结果为准。
const hint = computed(() => detectShareHint(input.value))

const isAllSelected = computed(() => songs.value.length > 0 && selectedIds.value.length === songs.value.length)
const isIndeterminate = computed(() => selectedIds.value.length > 0 && selectedIds.value.length < songs.value.length)

// 单曲链接只返回一首歌，界面用它切换成单曲卡片。
const singleSong = computed<SongInfo | null>(() => {
    if (result.value?.kind !== 'song') return null
    return (result.value.data as SongDetailResponse).song ?? null
})

const summary = computed(() => {
    const parsed = result.value
    if (!parsed) return null
    if (parsed.kind === 'song') {
        const song = (parsed.data as SongDetailResponse).song
        const qualities = (song.qualities ?? []).map((item) => item.quality).join(' / ')
        const meta = [
            song.album ? `专辑：${song.album}` : '',
            qualities ? `可用音质：${qualities}` : '暂无可用音质（可能需要会员）',
        ]
        return {
            title: song.title || '未知歌曲',
            subtitle: song.artist || '',
            cover: song.coverUrl || '',
            meta: meta.filter(Boolean).join(' · '),
        }
    }
    if (parsed.kind === 'album') {
        const album = (parsed.data as AlbumSongsResponse).album
        return {
            title: album?.name || '未命名专辑',
            subtitle: album?.artist ? `专辑艺人：${album.artist}` : '',
            cover: album?.coverUrl || '',
            meta: `${album?.publishDate ? `发行：${album.publishDate} · ` : ''}共 ${songs.value.length} 首`,
        }
    }
    const playlist = (parsed.data as PlaylistSongsResponse).playlist
    const meta = [`共 ${songs.value.length} 首`]
    if (playlist?.playCount) meta.push(`播放量：${formatPlayCount(playlist.playCount)}`)
    return {
        title: playlist?.name || '未命名歌单',
        subtitle: playlist?.creator ? `创建者：${playlist.creator}` : '',
        cover: playlist?.coverUrl || '',
        meta: meta.join(' · '),
    }
})

const logStore = useLogStore()
const { downloadSingle, batchDownload } = useDownloadActions()
const { openRelatedArtist, openSongAlbum } = useMusicNavigation()

function platformName(platform: string) {
    return PLATFORM_NAMES[platform] ?? platform
}

function kindName(kind: string) {
    return LINK_KIND_NAMES[kind as keyof typeof LINK_KIND_NAMES] ?? kind
}

function fill(url: string) {
    input.value = url
}

function toggleAll(checked: boolean) {
    selectedIds.value = checked ? songs.value.map((s) => s.mid) : []
}

function toggleSelect(songMid: string, selected: boolean) {
    if (selected) {
        if (!selectedIds.value.includes(songMid)) selectedIds.value.push(songMid)
    } else {
        selectedIds.value = selectedIds.value.filter((id) => id !== songMid)
    }
}

/** QQ 未登录时即使返回了音质也拉不到下载链接，提前提示而不是等下载失败。 */
async function warnIfQqLoggedOut() {
    const status = await getLoginStatus('qqmusic').catch(() => null)
    if (status && !status.logged_in) {
        loginNotice.value = 'QQ 音乐未登录，下载时可能拿不到链接'
    }
}

function reset() {
    input.value = ''
    loading.value = false
    errorMsg.value = ''
    result.value = null
    songs.value = []
    selectedIds.value = []
    loginNotice.value = ''
}

async function handleParse() {
    const value = input.value.trim()
    if (!value || loading.value) return

    loading.value = true
    errorMsg.value = ''
    result.value = null
    songs.value = []
    selectedIds.value = []

    try {
        // 输入是链接时服务端会自动识别平台，这里的平台只对裸 ID 生效。
        const parsed = await parseShareLink(value, fallbackPlatform.value)
        if (parsed.kind === 'song') {
            const song = (parsed.data as SongDetailResponse).song
            songs.value = song ? [song] : []
        } else {
            songs.value = (parsed.data as PlaylistSongsResponse | AlbumSongsResponse).songs ?? []
        }
        result.value = parsed
        if (!songs.value.length) {
            errorMsg.value = '解析成功，但该内容没有可下载的歌曲'
            logStore.push({
                level: 'warn',
                scope: 'parse',
                title: '解析成功但没有可用歌曲',
                detail: `${platformName(parsed.platform)} ${kindName(parsed.kind)} · ${summary.value?.title ?? value.slice(0, 60)}`,
                platform: parsed.platform,
            })
            return
        }
        logStore.push({
            level: 'success',
            scope: 'parse',
            title: `解析成功 · ${platformName(parsed.platform)} ${kindName(parsed.kind)}`,
            detail: `${summary.value?.title ?? ''}${summary.value?.title ? ' · ' : ''}${songs.value.length} 首歌曲`,
            platform: parsed.platform,
        })
        // 解析到哪个平台就只用哪个平台，不跨平台替换；只有 QQ 未登录时提醒一句。
        if (parsed.platform === 'qqmusic') {
            await warnIfQqLoggedOut()
        }
    } catch (e: any) {
        errorMsg.value = e?.message || String(e) || '解析失败'
        logStore.push({
            level: 'error',
            scope: 'parse',
            title: '解析失败',
            detail: `${errorMsg.value} · ${value.slice(0, 100)}`,
        })
    } finally {
        loading.value = false
    }
}

function onBatchDownload() {
    const selectedSongs = songs.value.filter((s) => selectedIds.value.includes(s.mid))
    if (selectedSongs.length > 0) batchDownload(selectedSongs)
}

// 搜索结果点进歌单时会带上 platform 与 id，这里直接解析，行为与粘贴 ID 一致。
async function loadFromQuery() {
    if (route.path !== '/playlist') return
    const qPlatform = route.query.platform as string | undefined
    const qId = route.query.id as string | undefined
    if (!qPlatform || !qId) return
    if (input.value === qId && result.value) return
    fallbackPlatform.value = qPlatform
    input.value = qId
    await handleParse()
}

onMounted(loadFromQuery)
onActivated(loadFromQuery)

watch(
    () => route.query.platform + '|' + route.query.id,
    (newVal, oldVal) => {
        if (newVal && newVal !== oldVal) loadFromQuery()
    }
)
</script>

<style scoped>
.parse-view {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
}

.parse-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 22px 24px 20px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
}

.parse-head {
    margin-bottom: 2px;
}

.parse-label {
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.14em;
    color: var(--text-tertiary);
}

.parse-hint {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--teal);
}

.parse-hint.is-muted {
    color: var(--text-tertiary);
}

.hint-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    flex-shrink: 0;
}

.parse-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
}

.fallback {
    display: flex;
    align-items: center;
    gap: 8px;
}

.fallback-label {
    font-size: 12.5px;
    color: var(--text-tertiary);
    white-space: nowrap;
}

.fallback-select {
    width: 132px;
}

.parse-buttons {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
}

.examples {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--border-light);
}

.examples-label {
    font-size: 12px;
    color: var(--text-tertiary);
}

.example-chip {
    padding: 4px 10px;
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 999px;
    cursor: pointer;
    transition: var(--transition);
}

.example-chip:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-light);
}

.state-wrapper {
    display: flex;
    justify-content: center;
    padding: 40px 0;
}

.result-info {
    display: flex;
    gap: 20px;
    align-items: center;
    padding: 22px 24px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
}

.result-cover {
    width: 92px;
    height: 92px;
    border-radius: 14px;
    object-fit: cover;
    flex-shrink: 0;
    background: var(--surface-sunken);
}

.result-cover.is-empty {
    background: var(--border-light);
}

.result-detail {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
}

.result-badges {
    display: flex;
    gap: 6px;
    margin-bottom: 8px;
}

.result-actions {
    margin-left: auto;
    flex-shrink: 0;
}

.badge {
    padding: 2px 8px;
    font-size: 11.5px;
    color: var(--text-secondary);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 999px;
}

.badge.is-accent {
    color: var(--accent);
    background: var(--accent-light);
    border-color: transparent;
}

.result-title {
    font-size: 21px;
    font-weight: 700;
    margin-bottom: 8px;
    color: var(--text-primary);
}

.result-sub,
.result-meta {
    color: var(--text-tertiary);
    font-size: 13.5px;
}

.result-meta {
    margin-top: 4px;
}

.parse-notice {
    margin-top: 8px;
    font-size: 12.5px;
    line-height: 1.6;
    color: var(--warning);
}

.parse-notice-link {
    margin-left: 8px;
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 2px;
}

.list-header {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
}

.count-text {
    font-size: 13px;
    color: var(--text-secondary);
}

.song-items {
    display: flex;
    flex-direction: column;
    gap: 14px;
}

/* 窄屏缩小信息区留白，封面保持比例，长文本自动换行 */
@media (max-width: 767px) {
    .parse-card {
        padding: 16px;
    }

    .result-info {
        align-items: flex-start;
        gap: 12px;
        padding: 14px;
    }

    .result-cover {
        width: 64px;
        height: 64px;
        border-radius: 10px;
    }

    .parse-buttons {
        width: 100%;
        justify-content: flex-end;
    }
}
</style>
