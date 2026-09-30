<template>
    <div class="dashboard-view">
        <div class="head-row">
            <div>
                <div class="page-title">系统监控仪表盘</div>
                <div class="page-sub">下载任务与曲库的整体概览</div>
            </div>
            <div class="head-right">
                <span class="status-badge" :class="connectionClass">
                    <span class="dot" />
                    {{ connectionText }}
                </span>
            </div>
        </div>

        <div class="dash-grid">
            <div class="dash-left">
                <div class="card">
                    <div class="card-head">
                        <div class="sec-title">最近下载</div>
                        <button class="link-more" type="button" @click="goTasks">查看全部</button>
                    </div>
                    <div class="table-wrap">
                        <table>
                            <colgroup>
                                <col style="width: 40%" />
                                <col style="width: 16%" />
                                <col style="width: 26%" />
                                <col style="width: 18%" />
                            </colgroup>
                            <thead>
                                <tr>
                                    <th>歌曲信息</th>
                                    <th>下载来源</th>
                                    <th>下载进度</th>
                                    <th>任务状态</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr v-for="task in recentTasks" :key="task.id">
                                    <td>
                                        <div class="cell-title">{{ task.songTitle }}</div>
                                        <div class="cell-sub">{{ task.artist || '未知歌手' }}</div>
                                    </td>
                                    <td class="cell-source">{{ platformLabel(task.platform) }}</td>
                                    <td>
                                        <div class="progress">
                                            <div class="bar">
                                                <i :class="progressFill(task)" :style="{ width: progressOf(task) + '%' }" />
                                            </div>
                                            <span class="pct">{{ progressOf(task) }}%</span>
                                        </div>
                                    </td>
                                    <td>
                                        <span class="badge" :class="statusBadge(task.status).cls">
                                            {{ statusBadge(task.status).label }}
                                        </span>
                                    </td>
                                </tr>
                                <tr v-if="recentTasks.length === 0">
                                    <td colspan="4" class="empty-cell">暂无下载任务</td>
                                </tr>
                            </tbody>
                        </table>
                    </div>
                </div>

                <div class="card log-card">
                    <div class="card-head">
                        <div class="sec-title">最近活动日志 (Recent Activity)</div>
                    </div>
                    <div class="log-list">
                        <div v-for="(log, index) in logs" :key="index" class="log-item"
                            :class="{ 'is-error': log.error }">
                            <span class="log-time">[{{ log.time }}]</span>
                            <span class="log-tag" :class="log.cls">[{{ log.tag }}]</span>
                            <span class="log-msg" :title="log.msg">{{ log.msg }}</span>
                        </div>
                        <div v-if="logs.length === 0" class="empty-cell">暂无活动记录</div>
                    </div>
                </div>
            </div>

            <div class="stat-col">
                <div class="stat-card">
                    <div class="stat-label">最近 7 天下载</div>
                    <div class="stat-value">{{ stats.recentDownloads }}</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">已下总量</div>
                    <div class="stat-value">{{ stats.completed }}</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">曲库数量</div>
                    <div class="stat-value">{{ libraryStore.files.length }}</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">曲库占用</div>
                    <div class="stat-value small">{{ formatFileSize(libraryStore.totalSize) }}</div>
                    <div class="stat-note">来源：下载目录中的音频文件</div>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, onActivated, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useTaskStore } from '../stores/taskStore'
import { useLibraryStore } from '../stores/libraryStore'
import { PLATFORMS } from '../config/platforms'
import { formatFileSize } from '../utils/format'
import type { TaskRecord, TaskStatus } from '../types'

const router = useRouter()
const taskStore = useTaskStore()
const libraryStore = useLibraryStore()

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

const logs = computed(() => {
    return [...taskStore.tasks]
        .sort((a, b) => b.addedAt - a.addedAt)
        .slice(0, 6)
        .map((task) => {
            const time = new Date(task.addedAt).toLocaleTimeString('zh-CN', { hour12: false })
            if (task.status === 'completed') {
                return { time, tag: 'INFO', cls: 'info', error: false, msg: `下载完成: ${task.songTitle} - ${task.artist}` }
            }
            if (task.status === 'error') {
                // 直接展示后端返回的具体失败原因，方便定位问题。
                const reason = task.errorMsg?.trim() || '未知原因'
                return {
                    time,
                    tag: 'ERROR',
                    cls: 'error',
                    error: true,
                    msg: `下载失败: ${task.songTitle} - ${task.artist}；原因：${reason}`,
                }
            }
            if (task.status === 'downloading') {
                return { time, tag: 'INFO', cls: 'info', error: false, msg: `下载中: ${task.songTitle} - ${task.artist}` }
            }
            return { time, tag: 'SYSTEM', cls: 'system', error: false, msg: `任务已加入队列: ${task.songTitle}` }
        })
})

function goTasks() {
    void router.push('/task')
}

onMounted(() => { void libraryStore.load() })
onActivated(() => { void libraryStore.load(true) })
</script>

<style scoped>
.dashboard-view {
    display: flex;
    flex-direction: column;
    min-height: 100%;
    min-width: 0;
}

.head-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
}

.page-title {
    font-size: 22px;
    font-weight: 700;
    color: var(--text-primary);
}

.page-sub {
    font-size: 13px;
    color: var(--text-secondary);
    margin-top: 4px;
}

.head-right {
    flex: none;
    padding-top: 4px;
}

.status-badge {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: var(--orange-weak, #fdf3e3);
    color: var(--orange, #d97706);
    border: 1px solid #f0dcb4;
    padding: 7px 16px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 600;
}

.status-badge.online {
    background: var(--green-weak, #e8f8ee);
    color: var(--green, #16a34a);
    border-color: #bfe8cc;
}

.status-badge .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: currentColor;
}

.dash-grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 300px;
    gap: 18px;
    margin-top: 24px;
    align-items: start;
}

.dash-left {
    display: flex;
    flex-direction: column;
    gap: 18px;
    min-width: 0;
}

.stat-col {
    display: flex;
    flex-direction: column;
    gap: 14px;
}

.stat-card {
    padding: 20px 22px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
}

.stat-label {
    font-size: 13px;
    color: var(--text-tertiary);
}

.stat-value {
    font-size: 34px;
    font-weight: 700;
    margin-top: 8px;
    letter-spacing: -0.5px;
    line-height: 1.15;
    color: var(--text-primary);
}

.stat-value.small {
    font-size: 24px;
}

.stat-note {
    font-size: 12.5px;
    color: var(--text-secondary);
    margin-top: 8px;
}

.card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
}

.card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 20px 14px;
}

.sec-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
}

.link-more {
    border: none;
    background: none;
    color: var(--accent);
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 6px;
}

.link-more:hover {
    background: var(--accent-light);
}

.table-wrap {
    overflow-x: auto;
}

table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
}

th {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-tertiary);
    text-align: left;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
    border-top: 1px solid var(--border);
    background: #fafbfc;
}

td {
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
    font-size: 14px;
    vertical-align: middle;
}

tbody tr:last-child td {
    border-bottom: none;
}

tbody tr:hover {
    background: #fafbfd;
}

.cell-title {
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.cell-sub {
    font-size: 12.5px;
    color: var(--text-tertiary);
    margin-top: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.cell-source {
    color: var(--text-secondary);
}

.progress {
    display: flex;
    align-items: center;
    gap: 10px;
}

.progress .bar {
    flex: 1;
    height: 7px;
    background: #eef0f3;
    border-radius: 999px;
    overflow: hidden;
}

.progress .bar i {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--accent);
}

.progress .bar i.green {
    background: var(--green, #16a34a);
}

.progress .bar i.red {
    background: var(--danger);
}

.progress .pct {
    font-size: 12.5px;
    color: var(--text-secondary);
    width: 40px;
    text-align: right;
    flex: none;
}

.badge {
    font-size: 12.5px;
    padding: 4px 12px;
    border-radius: 999px;
    font-weight: 500;
    display: inline-block;
    white-space: nowrap;
}

.badge.blue {
    background: var(--accent-light);
    color: var(--accent);
}

.badge.green {
    background: var(--green-weak, #e8f8ee);
    color: var(--green, #16a34a);
}

.badge.gray {
    background: #f0f1f3;
    color: var(--text-tertiary);
}

.badge.red {
    background: var(--danger-weak, #fff1f0);
    color: var(--danger);
}

.badge.orange {
    background: var(--orange-weak, #fdf3e3);
    color: var(--orange, #d97706);
}

.log-card {
    padding-bottom: 6px;
}

.log-list {
    font-family: var(--mono);
    font-size: 13px;
    display: flex;
    flex-direction: column;
    padding: 0 20px;
}

.log-item {
    display: flex;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px dashed #eef0f3;
    align-items: baseline;
    min-width: 0;
}

.log-item:last-child {
    border-bottom: none;
}

.log-time {
    color: var(--text-tertiary);
    flex: none;
}

.log-tag {
    flex: none;
    font-weight: 600;
    border-radius: 5px;
    padding: 2px 8px;
    font-size: 11.5px;
}

.log-tag.info {
    background: var(--accent-light);
    color: var(--accent);
}

.log-tag.warn {
    background: var(--orange-weak, #fdf3e3);
    color: var(--orange, #d97706);
}

.log-tag.error {
    background: var(--danger-weak, #fff1f0);
    color: var(--danger);
}

.log-tag.system {
    background: #f0f1f3;
    color: var(--text-secondary);
}

.log-msg {
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

.empty-cell {
    text-align: center;
    color: var(--text-tertiary);
    padding: 44px 18px;
    font-family: var(--font);
}

@media (max-width: 1080px) {
    .dash-grid {
        grid-template-columns: 1fr;
    }

    .stat-col {
        flex-direction: row;
        flex-wrap: wrap;
    }

    .stat-card {
        flex: 1;
        min-width: 160px;
    }
}
</style>
