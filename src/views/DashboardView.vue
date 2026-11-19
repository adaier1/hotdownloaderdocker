<template>
    <div class="dashboard-view">
        <header class="page-head rise" style="--i: 0">
            <div class="page-head-main">
                <div class="eyebrow">Overview · 总览</div>
                <h1 class="page-title">下载总览</h1>
                <p class="page-sub">下载任务、曲库与运行状态的同一张纸面。数据更新于 {{ updatedAt }}。</p>
            </div>
            <div class="page-head-actions">
                <span class="live-chip" :class="connectionClass">
                    <span class="dot" />
                    {{ connectionText }}
                </span>
                <button class="btn-ink" type="button" @click="goTasks">
                    查看任务队列
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"
                        stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                        <path d="M5 12h14M13 6l6 6-6 6" />
                    </svg>
                </button>
            </div>
        </header>

        <!-- 指标条：一张卡、四条竖线分隔，像仪表盘刻度 -->
        <section class="metric-strip rise" style="--i: 1" aria-label="关键指标">
            <div class="metric">
                <div class="metric-label">近 7 天完成</div>
                <div class="metric-value">{{ stats.recentDownloads }}</div>
                <div class="metric-note">首歌曲落盘</div>
            </div>
            <div class="metric">
                <div class="metric-label">累计完成</div>
                <div class="metric-value">{{ stats.completed }}</div>
                <div class="metric-note">含历史任务</div>
            </div>
            <div class="metric">
                <div class="metric-label">曲库文件</div>
                <div class="metric-value">{{ libraryStore.files.length }}</div>
                <div class="metric-note">下载目录内音频</div>
            </div>
            <div class="metric">
                <div class="metric-label">占用空间</div>
                <div class="metric-value small">{{ formatFileSize(libraryStore.totalSize) }}</div>
                <div class="metric-note">平均 {{ avgSize }}</div>
            </div>
        </section>

        <div class="dash-grid">
            <div class="dash-main">
                <!-- 最近下载：行式列表，不是表格 -->
                <section class="card list-card rise" style="--i: 2">
                    <div class="card-head">
                        <div class="sec-title">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"
                                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3" />
                            </svg>
                            最近下载
                        </div>
                        <button class="link-more" type="button" @click="goTasks">全部任务 →</button>
                    </div>

                    <div class="dl-list">
                        <div v-for="(task, index) in recentTasks" :key="task.id" class="dl-row">
                            <span class="dl-index">{{ String(index + 1).padStart(2, '0') }}</span>
                            <div class="dl-cover" :class="'p-' + task.platform">
                                <img v-if="task.coverUrl" :src="task.coverUrl" alt="" loading="lazy" />
                                <span v-else>{{ platformLabel(task.platform) }}</span>
                            </div>
                            <div class="dl-meta">
                                <div class="dl-title">{{ task.songTitle }}</div>
                                <div class="dl-sub">{{ task.artist || '未知歌手' }}</div>
                            </div>
                            <span class="dl-source">{{ platformLabel(task.platform) }}</span>
                            <div class="dl-progress">
                                <div class="bar">
                                    <i :class="progressFill(task)" :style="{ width: progressOf(task) + '%' }" />
                                </div>
                                <span class="pct">{{ progressOf(task) }}%</span>
                            </div>
                            <span class="badge" :class="statusBadge(task.status).cls">
                                {{ statusBadge(task.status).label }}
                            </span>
                        </div>

                        <div v-if="recentTasks.length === 0" class="empty-state">
                            <div class="empty-title">还没有下载任务</div>
                            <div class="empty-sub">去搜索页挑一首歌，或者粘贴一个歌单链接。</div>
                        </div>
                    </div>
                </section>

                <!-- 活动日志：等宽时间戳，像终端但更安静 -->
                <section class="card log-card rise" style="--i: 4">
                    <div class="card-head">
                        <div class="sec-title">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"
                                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                                <rect x="3" y="4" width="18" height="16" rx="3.5" />
                                <path d="m7.5 9.5 2.6 2.5-2.6 2.5M12.8 14.5h4" />
                            </svg>
                            运行日志
                        </div>
                        <button class="link-more" type="button" @click="goLogs">全部日志 →</button>
                    </div>
                    <div class="log-list">
                        <div v-for="log in recentLogs" :key="log.id" class="log-item"
                            :class="{ 'is-error': log.error }">
                            <span class="log-time">{{ log.time }}</span>
                            <span class="log-tag" :class="log.cls">{{ log.tag }}</span>
                            <span class="log-msg" :title="log.msg">{{ log.msg }}</span>
                        </div>
                        <div v-if="recentLogs.length === 0" class="empty-cell">暂无日志，解析或下载后会在这里留下记录</div>
                    </div>
                </section>
            </div>

            <aside class="dash-side">
                <!-- 趋势：细柱状图，参考图里的报修/解决对照 -->
                <section class="card chart-card rise" style="--i: 3">
                    <div class="card-head">
                        <div class="sec-title">下载趋势</div>
                        <div class="legend">
                            <span class="lg"><i class="ok" />完成</span>
                            <span class="lg"><i class="err" />失败</span>
                        </div>
                    </div>

                    <div class="chart-headline">
                        <span class="chart-big">{{ weekTotal }}</span>
                        <span class="chart-unit">首 / 近 7 天</span>
                    </div>

                    <div class="chart-body">
                        <div class="chart-grid">
                            <div v-for="day in week" :key="day.key" class="chart-col">
                                <div class="bars">
                                    <span class="bar ok" :style="{ height: barHeight(day.ok) }"
                                        :title="`${day.full} 完成 ${day.ok} 首`" />
                                    <span class="bar err" :style="{ height: barHeight(day.err) }"
                                        :title="`${day.full} 失败 ${day.err} 首`" />
                                </div>
                                <span class="col-label">{{ day.label }}</span>
                            </div>
                        </div>
                        <div v-if="weekTotal === 0" class="chart-empty">最近 7 天没有下载记录</div>
                    </div>

                    <div class="chart-foot">
                        <span>完成 <b>{{ weekOkTotal }}</b> · 失败 <b>{{ weekErrTotal }}</b></span>
                        <span>单日峰值 <b>{{ weekPeak }}</b> 首</span>
                    </div>
                </section>

                <!-- 曲库占用 -->
                <section class="card storage-card rise" style="--i: 5">
                    <div class="card-head">
                        <div class="sec-title">曲库</div>
                        <button class="link-more" type="button" @click="goLibrary">打开 →</button>
                    </div>
                    <div class="storage-value">{{ formatFileSize(libraryStore.totalSize) }}</div>
                    <div class="storage-bar">
                        <i class="seg-a" :style="{ width: losslessPercent + '%' }" />
                        <i class="seg-b" :style="{ width: 100 - losslessPercent + '%' }" />
                    </div>
                    <div class="storage-legend">
                        <span><i class="seg-a" />无损 {{ losslessCount }}</span>
                        <span><i class="seg-b" />其他 {{ libraryStore.files.length - losslessCount }}</span>
                    </div>
                    <div class="storage-path" :title="libraryStore.directory || '未设置'">
                        {{ libraryStore.directory || '下载目录未设置' }}
                    </div>
                </section>
            </aside>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useTaskStore } from '../stores/taskStore'
import { useLibraryStore } from '../stores/libraryStore'
import { useLogStore } from '../stores/logStore'
import { PLATFORMS } from '../config/platforms'
import { formatFileSize } from '../utils/format'
import type { TaskRecord, TaskStatus } from '../types'

const router = useRouter()
const taskStore = useTaskStore()
const libraryStore = useLibraryStore()
const logStore = useLogStore()

const updatedAt = ref(
    new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false })
)

const recentTasks = computed(() =>
    [...taskStore.tasks]
        .sort((a, b) => b.addedAt - a.addedAt)
        .slice(0, 6)
)

const stats = computed(() => {
    const weekAgo = Date.now() - 7 * 24 * 60 * 60 * 1000
    let completed = 0
    let recentDownloads = 0
    for (const task of taskStore.tasks) {
        if (task.status !== 'completed') continue
        completed++
        if (task.addedAt >= weekAgo) recentDownloads++
    }
    return { completed, recentDownloads }
})

const avgSize = computed(() => {
    const count = libraryStore.files.length
    if (!count) return '—'
    return formatFileSize(Math.round(libraryStore.totalSize / count))
})

const connectionText = computed(() => {
    switch (taskStore.connectionStatus) {
        case 'connected': return '在线'
        case 'connecting': return '连接中'
        case 'reconnecting': return '重连中'
        default: return '离线'
    }
})

const connectionClass = computed(() => ({
    online: taskStore.connectionStatus === 'connected',
    offline: taskStore.connectionStatus === 'disconnected',
}))

/* ---------- 近 7 天趋势 ---------- */

const WEEK_LABELS = ['日', '一', '二', '三', '四', '五', '六']

const week = computed(() => {
    // 以每天 0 点为界统计任务新增（完成 / 失败），不引入额外请求。
    const days: Array<{ key: string; label: string; full: string; ok: number; err: number }> = []
    const today = new Date()
    today.setHours(0, 0, 0, 0)

    for (let offset = 6; offset >= 0; offset--) {
        const day = new Date(today)
        day.setDate(today.getDate() - offset)
        const start = day.getTime()
        const end = start + 24 * 60 * 60 * 1000
        let ok = 0
        let err = 0
        for (const task of taskStore.tasks) {
            if (task.addedAt < start || task.addedAt >= end) continue
            if (task.status === 'completed') ok++
            else if (task.status === 'error') err++
        }
        days.push({
            key: String(start),
            label: WEEK_LABELS[day.getDay()],
            full: `${day.getMonth() + 1} 月 ${day.getDate()} 日`,
            ok,
            err,
        })
    }
    return days
})

const weekPeak = computed(() => Math.max(0, ...week.value.map((d) => Math.max(d.ok, d.err))))
const weekTotal = computed(() => week.value.reduce((sum, d) => sum + d.ok + d.err, 0))
const weekOkTotal = computed(() => week.value.reduce((sum, d) => sum + d.ok, 0))
const weekErrTotal = computed(() => week.value.reduce((sum, d) => sum + d.err, 0))

/** 柱高按周峰值归一化，但至少按 2 首为满格，避免"一天一首"顶满整个画布。 */
const weekScale = computed(() => Math.max(1, weekPeak.value))

function barHeight(value: number): string {
    if (value <= 0) return '0%'
    // 顶部留 12% headroom，避免峰值柱顶到卡片边缘
    return `${Math.max(8, Math.round((value / weekScale.value) * 88))}%`
}

/* ---------- 曲库构成 ---------- */

const LOSSLESS_EXT = new Set(['flac', 'ape', 'wav', 'alac'])
const losslessCount = computed(
    () => libraryStore.files.filter((file) => LOSSLESS_EXT.has(file.extension?.toLowerCase?.() ?? '')).length
)
const losslessPercent = computed(() => {
    const total = libraryStore.files.length
    if (!total) return 0
    return Math.round((losslessCount.value / total) * 100)
})

/* ---------- 状态映射 ---------- */

const STATUS_MAP: Record<TaskStatus, { label: string; cls: string }> = {
    waiting: { label: '等待中', cls: 'gray' },
    downloading: { label: '下载中', cls: 'blue' },
    paused: { label: '暂停', cls: 'gray' },
    completed: { label: '已完成', cls: 'green' },
    processing: { label: '处理中', cls: 'orange' },
    interrupted: { label: '已中断', cls: 'orange' },
    error: { label: '错误', cls: 'red' },
}

function statusBadge(status: TaskStatus) {
    return STATUS_MAP[status] ?? { label: status, cls: 'gray' }
}

function platformLabel(platform: string): string {
    return PLATFORMS.find((item) => item.key === platform)?.label ?? platform
}

function progressOf(task: TaskRecord): number {
    if (task.status === 'completed') return 100
    if (!task.fileSize) return 0
    return Math.min(100, Math.round((task.downloaded / task.fileSize) * 100))
}

function progressFill(task: TaskRecord): string {
    if (task.status === 'error') return 'red'
    if (task.status === 'completed') return 'green'
    return ''
}

const scopeName: Record<string, string> = {
    parse: '解析',
    download: '下载',
    account: '账号',
    system: '系统',
}

/** 总览只展示最近 5 条，完整流水在日志页。 */
const recentLogs = computed(() =>
    logStore.recent(5).map((entry) => ({
        id: entry.id,
        time: new Date(entry.at).toLocaleTimeString('zh-CN', { hour12: false }),
        tag: scopeName[entry.scope] ?? '日志',
        cls: entry.level === 'success' ? 'info' : entry.level === 'info' ? 'system' : entry.level,
        msg: entry.detail ? `${entry.title} · ${entry.detail}` : entry.title,
        error: entry.level === 'error',
    }))
)

function goTasks() {
    void router.push('/task')
}

function goLibrary() {
    void router.push('/library')
}

function goLogs() {
    void router.push('/logs')
}

onMounted(() => {
    logStore.ensureLoaded()
    void libraryStore.load()
})
onActivated(() => {
    logStore.ensureLoaded()
    void libraryStore.load(true)
    updatedAt.value = new Date().toLocaleTimeString('zh-CN', {
        hour: '2-digit',
        minute: '2-digit',
        hour12: false,
    })
})
</script>

<style scoped>
.dashboard-view {
    display: flex;
    flex-direction: column;
    min-height: 100%;
    min-width: 0;
}

/* ---------- 指标条 ---------- */
.metric-strip {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
}

.metric {
    padding: 20px 22px 22px;
    position: relative;
    min-width: 0;
}

.metric + .metric::before {
    content: '';
    position: absolute;
    left: 0;
    top: 20px;
    bottom: 20px;
    width: 1px;
    background: var(--border-light);
}

.metric-label {
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.13em;
    text-transform: uppercase;
    color: var(--text-tertiary);
}

.metric-value {
    margin-top: 12px;
    font-family: var(--font-display);
    font-size: 32px;
    font-weight: 500;
    letter-spacing: -0.035em;
    line-height: 1.05;
    color: var(--text-primary);
}

.metric-value.small {
    font-size: 24px;
    letter-spacing: -0.025em;
    padding-top: 6px;
}

.metric-note {
    margin-top: 8px;
    font-size: 11.5px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

/* ---------- 主栅格 ---------- */
.dash-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: 16px;
    margin-top: 16px;
    align-items: start;
}

.dash-main,
.dash-side {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
}

.card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
}

/* ---------- 下载列表 ---------- */
.dl-list {
    padding: 0 10px 8px;
}

.dl-row {
    display: grid;
    grid-template-columns: 24px 38px minmax(0, 1fr) auto 108px auto;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    border-radius: 11px;
    transition: background var(--transition);
}

.dl-row:hover {
    background: var(--surface-sunken);
}

.dl-index {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
}

.dl-cover {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-light);
    display: grid;
    place-items: center;
    overflow: hidden;
    flex: none;
}

.dl-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
}

.dl-cover span {
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.05em;
    color: var(--text-tertiary);
}

.dl-cover.p-qqmusic {
    background: #eff2e9;
}

.dl-cover.p-kugou {
    background: #eef1f6;
}

.dl-cover.p-netease {
    background: #f9ecec;
}

.dl-cover.p-kuwo {
    background: #f6f0e6;
}

.dl-meta {
    min-width: 0;
}

.dl-title {
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.45;
}

.dl-sub {
    font-size: 11.5px;
    color: var(--text-tertiary);
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.dl-source {
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.06em;
    color: var(--text-tertiary);
    padding: 3px 7px;
    border-radius: 5px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-light);
}

.dl-progress {
    display: flex;
    align-items: center;
    gap: 8px;
}

.dl-progress .bar {
    flex: 1;
    height: 3px;
    background: var(--border-light);
    border-radius: 999px;
    overflow: hidden;
}

.dl-progress .bar i {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
    transition: width 0.4s var(--ease-out);
}

.dl-progress .bar i.green {
    background: var(--green);
}

.dl-progress .bar i.red {
    background: var(--danger);
}

.dl-progress .pct {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-tertiary);
    width: 32px;
    text-align: right;
    flex: none;
    font-variant-numeric: tabular-nums;
}

.badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    padding: 3px 9px;
    border-radius: 999px;
    font-weight: 500;
    white-space: nowrap;
    justify-self: end;
}

.badge::before {
    content: '';
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: currentColor;
    opacity: 0.8;
}

.badge.blue {
    background: var(--blue-weak);
    color: var(--blue);
}

.badge.green {
    background: var(--green-weak);
    color: var(--green);
}

.badge.gray {
    background: var(--surface-sunken);
    color: var(--text-tertiary);
}

.badge.red {
    background: var(--danger-weak);
    color: var(--danger);
}

.badge.orange {
    background: var(--orange-weak);
    color: var(--orange);
}

.empty-state {
    padding: 40px 16px 44px;
    text-align: center;
}

.empty-title {
    font-size: 13.5px;
    color: var(--text-secondary);
}

.empty-sub {
    margin-top: 6px;
    font-size: 12px;
    color: var(--text-tertiary);
}

/* ---------- 趋势图 ---------- */
.legend {
    display: flex;
    gap: 14px;
    font-size: 11px;
    color: var(--text-tertiary);
}

.legend .lg {
    display: inline-flex;
    align-items: center;
    gap: 5px;
}

.legend i {
    width: 6px;
    height: 6px;
    border-radius: 2px;
}

.legend i.ok {
    background: var(--green);
}

.legend i.err {
    background: var(--danger);
}

.chart-headline {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 2px 22px 12px;
}

.chart-big {
    font-family: var(--font-display);
    font-size: 30px;
    font-weight: 500;
    letter-spacing: -0.035em;
    line-height: 1;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
}

.chart-unit {
    font-size: 11.5px;
    color: var(--text-tertiary);
}

.chart-body {
    position: relative;
    padding: 4px 22px 0;
}

.chart-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 6px;
    height: 112px;
    /* 三条极淡的水平刻度线，替代图表库的网格 */
    background-image: linear-gradient(to bottom, var(--border-light) 1px, transparent 1px);
    background-size: 100% 33.33%;
    background-position: 0 0;
}

.chart-col {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 100%;
    position: relative;
}

.bars {
    flex: 1;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    gap: 3px;
    width: 100%;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
}

.bar {
    width: 5px;
    border-radius: 999px;
    display: block;
    min-height: 0;
    transition: height 0.45s var(--ease-out), opacity var(--transition);
}

.bar.ok {
    background: var(--green);
}

.bar.err {
    background: var(--danger);
}

.bar:hover {
    opacity: 0.65;
}

.col-label {
    font-family: var(--mono);
    font-size: 10px;
    color: var(--text-tertiary);
    letter-spacing: 0.04em;
    flex: none;
}

.chart-empty {
    position: absolute;
    inset: 44px 22px 30px;
    display: grid;
    place-items: center;
    font-size: 12px;
    color: var(--text-tertiary);
    background: rgba(255, 255, 255, 0.6);
    border-radius: 10px;
}

.chart-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 22px 16px;
    margin-top: 6px;
    border-top: 1px solid var(--border-light);
    font-size: 11.5px;
    color: var(--text-tertiary);
}

.chart-foot b {
    font-family: var(--mono);
    font-weight: 500;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
}

/* ---------- 曲库卡 ---------- */
.storage-value {
    padding: 2px 22px 0;
    font-family: var(--font-display);
    font-size: 30px;
    font-weight: 500;
    letter-spacing: -0.035em;
    color: var(--text-primary);
}

.storage-bar {
    display: flex;
    gap: 3px;
    height: 5px;
    margin: 16px 22px 12px;
    border-radius: 999px;
    overflow: hidden;
    background: var(--border-light);
}

.storage-bar i {
    display: block;
    height: 100%;
    transition: width 0.5s var(--ease-out);
}

.seg-a {
    background: var(--accent);
}

.seg-b {
    background: #c9c7b8;
}

.storage-legend {
    display: flex;
    gap: 18px;
    padding: 0 22px;
    font-size: 11.5px;
    color: var(--text-secondary);
}

.storage-legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
}

.storage-legend i {
    width: 6px;
    height: 6px;
    border-radius: 2px;
}

.storage-path {
    margin: 16px 22px 18px;
    padding: 9px 11px;
    border-radius: 8px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-light);
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

/* ---------- 日志 ---------- */
.log-list {
    display: flex;
    flex-direction: column;
    padding: 0 22px 16px;
}

.log-item {
    display: flex;
    gap: 12px;
    padding: 9px 0;
    border-bottom: 1px solid var(--border-light);
    align-items: baseline;
    min-width: 0;
}

.log-item:last-child {
    border-bottom: none;
}

.log-time {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-tertiary);
    flex: none;
    font-variant-numeric: tabular-nums;
}

.log-tag {
    flex: none;
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.06em;
    border-radius: 4px;
    padding: 2px 6px;
    background: var(--surface-sunken);
    color: var(--text-tertiary);
}

.log-tag.info {
    background: var(--green-weak);
    color: var(--green);
}

.log-tag.error {
    background: var(--danger-weak);
    color: var(--danger);
}

.log-tag.warn {
    background: var(--orange-weak);
    color: var(--orange);
}

.log-tag.system {
    background: var(--surface-sunken);
    color: var(--text-tertiary);
}

.log-msg {
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
}

/* 失败日志完整展示原因，不截断 */
.log-item.is-error .log-msg {
    white-space: normal;
    overflow: visible;
    overflow-wrap: anywhere;
    color: var(--danger);
}

/* ---------- 响应式 ---------- */
@media (max-width: 1180px) {
    .dash-grid {
        grid-template-columns: minmax(0, 1fr) 300px;
    }

    .dl-row {
        grid-template-columns: 22px 38px minmax(0, 1fr) auto;
    }

    .dl-progress,
    .dl-source {
        display: none;
    }
}

@media (max-width: 960px) {
    .dash-grid {
        grid-template-columns: 1fr;
    }

    .metric-strip {
        grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .metric:nth-child(3)::before {
        display: none;
    }
}

@media (max-width: 767px) {
    .page-title {
        font-size: 24px;
    }

    .metric {
        padding: 16px 16px 18px;
    }

    .metric-value {
        font-size: 26px;
    }

    .dl-row {
        grid-template-columns: 38px minmax(0, 1fr) auto;
        gap: 12px;
        padding: 10px 8px;
    }

    .dl-index {
        display: none;
    }

    .card-head {
        padding: 16px 16px 10px;
    }

    .log-list {
        padding: 0 16px 12px;
    }
}
</style>
