<template>
    <div class="log-view">
        <header class="page-head rise" style="--i: 0">
            <div class="page-head-main">
                <div class="eyebrow">Logs · 运行日志</div>
                <h1 class="page-title">日志</h1>
                <p class="page-sub">
                    解析、下载与账号事件的流水。只保留最近 {{ MAX_ENTRIES }} 条，超出后自动丢弃。
                </p>
            </div>
            <div class="page-head-actions">
                <button class="btn-line danger" type="button" :disabled="logStore.entries.length === 0"
                    @click="confirmClear">
                    清空日志
                </button>
            </div>
        </header>

        <!-- 筛选与会话统计 -->
        <div class="log-toolbar rise" style="--i: 1">
            <div class="segmented" role="tablist" aria-label="日志筛选">
                <button v-for="tab in TABS" :key="tab.key" type="button" role="tab" class="seg"
                    :class="{ active: activeTab === tab.key }" :aria-selected="activeTab === tab.key"
                    @click="activeTab = tab.key">
                    <span>{{ tab.label }}</span>
                    <span class="seg-count">{{ countOf(tab.key) }}</span>
                </button>
            </div>
            <span class="retention">
                已保留 <b>{{ logStore.entries.length }}</b> / {{ MAX_ENTRIES }}
            </span>
        </div>

        <section class="card log-card rise" style="--i: 2">
            <template v-if="groups.length > 0">
                <div v-for="group in groups" :key="group.key" class="day-group">
                    <div class="day-label">
                        <span>{{ group.label }}</span>
                        <span class="day-count">{{ group.items.length }}</span>
                    </div>

                    <div v-for="entry in group.items" :key="entry.id" class="log-row" :class="entry.level">
                        <span class="log-time">{{ timeOf(entry.at) }}</span>
                        <span class="log-dot" aria-hidden="true" />
                        <span class="log-scope">{{ scopeLabel(entry.scope) }}</span>
                        <div class="log-body">
                            <div class="log-title">{{ entry.title }}</div>
                            <div v-if="entry.detail" class="log-detail">{{ entry.detail }}</div>
                        </div>
                        <span v-if="entry.platform" class="log-platform">
                            {{ platformLabel(entry.platform) }}
                        </span>
                        <span v-else class="log-platform" />
                    </div>
                </div>
            </template>

            <div v-else class="empty-state">
                <div class="empty-title">还没有日志</div>
                <div class="empty-sub">
                    {{ logStore.entries.length > 0 ? '当前筛选条件下没有记录' : '解析一个歌单或下载一首歌，这里就会留下流水。' }}
                </div>
            </div>
        </section>

        <p class="log-note rise" style="--i: 3">
            日志保存在本机，不上传服务端。清空后不可恢复。
        </p>
    </div>
</template>

<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue'
import { useDialog } from 'naive-ui'
import { MAX_LOG_ENTRIES, useLogStore, type LogEntry, type LogScope } from '../stores/logStore'
import { PLATFORMS } from '../config/platforms'

const MAX_ENTRIES = MAX_LOG_ENTRIES

const logStore = useLogStore()
const dialog = useDialog()

type FilterKey = 'all' | 'parse' | 'download' | 'error'

const TABS: Array<{ key: FilterKey; label: string }> = [
    { key: 'all', label: '全部' },
    { key: 'parse', label: '解析' },
    { key: 'download', label: '下载' },
    { key: 'error', label: '仅失败' },
]

const activeTab = ref<FilterKey>('all')

const filtered = computed<LogEntry[]>(() => {
    const list = logStore.entries
    switch (activeTab.value) {
        case 'parse': return list.filter((entry) => entry.scope === 'parse')
        case 'download': return list.filter((entry) => entry.scope === 'download')
        case 'error': return list.filter((entry) => entry.level === 'error')
        default: return list
    }
})

function countOf(key: FilterKey): number {
    if (key === 'all') return logStore.entries.length
    if (key === 'error') return logStore.errorCount
    const scope = key as LogScope
    return logStore.entries.filter((entry) => entry.scope === scope).length
}

/** 按自然日分组，避免长列表里时间失去参照。 */
const groups = computed(() => {
    const buckets: Array<{ key: string; label: string; items: LogEntry[] }> = []
    for (const entry of filtered.value) {
        const date = new Date(entry.at)
        const key = `${date.getFullYear()}-${date.getMonth() + 1}-${date.getDate()}`
        let bucket = buckets.find((item) => item.key === key)
        if (!bucket) {
            bucket = { key, label: dayLabel(date), items: [] }
            buckets.push(bucket)
        }
        bucket.items.push(entry)
    }
    return buckets
})

function dayLabel(date: Date): string {
    const today = new Date()
    const startOfToday = new Date(today.getFullYear(), today.getMonth(), today.getDate()).getTime()
    const startOfDay = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime()
    const diffDays = Math.round((startOfToday - startOfDay) / 86400000)
    if (diffDays === 0) return '今天'
    if (diffDays === 1) return '昨天'
    return `${date.getMonth() + 1} 月 ${date.getDate()} 日`
}

function timeOf(at: number): string {
    return new Date(at).toLocaleTimeString('zh-CN', { hour12: false })
}

const SCOPE_LABELS: Record<LogScope, string> = {
    parse: '解析',
    download: '下载',
    account: '账号',
    system: '系统',
}

function scopeLabel(scope: LogScope): string {
    return SCOPE_LABELS[scope] ?? scope
}

function platformLabel(platform: string): string {
    return PLATFORMS.find((item) => item.key === platform)?.label ?? platform.toUpperCase()
}

function confirmClear() {
    dialog.warning({
        title: '清空日志',
        content: `确定清空全部 ${logStore.entries.length} 条日志吗？此操作不可恢复，不影响任务与文件。`,
        positiveText: '清空',
        negativeText: '取消',
        onPositiveClick: () => logStore.clear(),
    })
}

onMounted(() => logStore.ensureLoaded())
onActivated(() => logStore.ensureLoaded())
</script>

<style scoped>
.log-view {
    display: flex;
    flex-direction: column;
    min-height: 100%;
    min-width: 0;
}

.btn-line.danger {
    color: var(--danger);
    border-color: #e8d2cd;
}

.btn-line.danger:disabled {
    opacity: 0.4;
}

/* 筛选条 */
.log-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 14px;
}

.segmented {
    display: flex;
    flex: 0 1 auto;
    min-width: 0;
    align-items: center;
    gap: 2px;
    padding: 4px;
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
    color: rgba(255, 255, 255, 0.62);
}

.retention {
    margin-left: auto;
    font-family: var(--mono);
    font-size: 10.5px;
    letter-spacing: 0.04em;
    color: var(--text-tertiary);
}

.retention b {
    font-weight: 500;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
}

/* 日志列表 */
.log-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
    padding-bottom: 6px;
}

.day-group + .day-group .day-label {
    border-top: 1px solid var(--border-light);
}

.day-label {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 22px 6px;
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-tertiary);
}

.day-count {
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--surface-sunken);
    color: var(--text-tertiary);
    font-size: 9.5px;
    letter-spacing: 0.04em;
}

.log-row {
    display: grid;
    grid-template-columns: 66px 7px 34px minmax(0, 1fr) 40px;
    align-items: start;
    gap: 12px;
    padding: 10px 22px;
    transition: background var(--transition);
}

.log-row:hover {
    background: var(--surface-hover);
}

.log-time {
    font-family: var(--mono);
    font-size: 10.5px;
    line-height: 1.7;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    padding-top: 1px;
}

.log-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    margin-top: 7px;
    background: var(--text-tertiary);
}

.log-row.success .log-dot {
    background: var(--green);
}

.log-row.warn .log-dot {
    background: var(--orange);
}

.log-row.error .log-dot {
    background: var(--danger);
}

.log-row.info .log-dot {
    background: var(--accent);
    opacity: 0.35;
}

.log-scope {
    font-size: 11px;
    line-height: 1.7;
    color: var(--text-tertiary);
    white-space: nowrap;
    padding-top: 1px;
}

.log-row.error .log-scope {
    color: var(--danger);
}

.log-body {
    min-width: 0;
}

.log-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.65;
    overflow-wrap: anywhere;
}

.log-row.error .log-title {
    color: var(--danger);
}

.log-detail {
    margin-top: 2px;
    font-family: var(--mono);
    font-size: 10.5px;
    line-height: 1.7;
    color: var(--text-tertiary);
    overflow-wrap: anywhere;
}

.log-platform {
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.06em;
    color: var(--text-tertiary);
    text-align: right;
    padding-top: 4px;
}

.log-platform.is-empty {
    opacity: 0.4;
}

.empty-state {
    padding: 64px 22px 68px;
    text-align: center;
}

.empty-title {
    font-size: 14px;
    color: var(--text-secondary);
}

.empty-sub {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-tertiary);
}

.log-note {
    margin-top: 14px;
    font-size: 11.5px;
    color: var(--text-tertiary);
    text-align: center;
}

@media (max-width: 767px) {
    .log-row {
        grid-template-columns: 62px 7px minmax(0, 1fr);
        gap: 10px;
        padding: 10px 16px;
    }

    .log-scope,
    .log-platform {
        display: none;
    }

    .day-label {
        padding: 12px 16px 4px;
    }

    .seg {
        height: 36px;
    }
}
</style>
