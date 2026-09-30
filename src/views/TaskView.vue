<template>
    <div class="task-view">
        <TaskTabs v-model:activeTab="activeTab" :counts="tabCounts" />

        <!-- 批量操作栏：按当前标签页显示可用的一键操作 -->
        <div v-if="showToolbar" class="task-toolbar">
            <!-- 中断恢复与错误重试按任务状态分别选择，实际规则交给 Rust。 -->
            <n-button v-if="(activeTab === 'error' && tabCounts.error > 0) ||
                (activeTab === 'interrupted' && tabCounts.interrupted > 0)" size="small" type="primary"
                :loading="retryingAll" :disabled="retryingAll" @click="handleRetryAll">
                {{ activeTab === 'interrupted' ? '恢复全部中断任务' : '全部重试' }}
                （{{ activeTab === 'interrupted' ? tabCounts.interrupted : tabCounts.error }}）
            </n-button>
            <span v-if="activeTab === 'error' || activeTab === 'interrupted'" class="task-toolbar-hint">
                会依次重新入队，实际同时下载数量由“最大并发数”决定
            </span>

            <!-- 清除所有已下载（已完成）的任务记录：仅移除记录，不影响磁盘文件 -->
            <n-button v-if="canClearCompleted" size="small" type="warning" :loading="clearing" :disabled="clearing"
                @click="confirmClearCompleted">
                清除所有已下载的任务（{{ tabCounts.completed }}）
            </n-button>

            <!-- 清除所有历史任务（含进行中的任务，会被取消） -->
            <n-button v-if="canClearAll" size="small" type="error" :loading="clearing" :disabled="clearing"
                @click="confirmClearAll">
                清除所有历史任务（{{ tabCounts.total }}）
            </n-button>
        </div>

        <div class="table-card">
            <TaskTable :tasks="pagedTasks" :selectedRowKeys="selectedRowKeys"
                @update:selectedRowKeys="selectedRowKeys = $event" @action="handleAction" />
        </div>

        <!-- 任务数量可能很大，只渲染当前页，避免一次性创建成千上万个 DOM/组件导致卡死 -->
        <div v-if="filteredTasks.length > pageSize" class="task-pagination">
            <n-pagination v-model:page="page" :page-size="pageSize" :item-count="filteredTasks.length"
                :page-slot="5" />
        </div>

        <TaskBatchActions :selectedCount="selectedRowKeys.length" @clear="handleBatchClear" />
    </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { openFileLocation } from '../api/fileApi'
import { NPagination, NButton, useDialog, useNotification } from 'naive-ui'
import { useTaskStore } from '../stores/taskStore'
import { useSettingsStore } from '../stores/settingsStore'
import { useDownloadActions } from '../composables/useDownloadActions'
import TaskTabs from '../components/task/TaskTabs.vue'
import TaskTable from '../components/task/TaskTable.vue'
import TaskBatchActions from '../components/task/TaskBatchActions.vue'
import type { TaskAction, TaskActionExtra } from '../components/task/TaskRowActions'

const taskStore = useTaskStore()
const settingsStore = useSettingsStore()
const { retryTask } = useDownloadActions()
const notification = useNotification()
const dialog = useDialog()

const activeTab = ref('all')
const selectedRowKeys = ref<string[]>([])
const retryingAll = ref(false)
const clearing = ref(false)

// 分页：任务列表可能包含上千条记录（尤其是“全部/已完成”），
// 一次渲染全部任务会创建大量组件实例并频繁重渲染，是崩溃与卡顿的主因之一。
const page = ref(1)
const pageSize = ref(50)

const tabCounts = computed(() => {
    const counts = {
        total: 0,
        waiting: 0,
        downloading: 0,
        paused: 0,
        completed: 0,
        interrupted: 0,
        error: 0,
    }
    for (const task of taskStore.tasks) {
        counts.total++
        if (task.status === 'waiting') counts.waiting++
        else if (task.status === 'downloading') counts.downloading++
        else if (task.status === 'paused') counts.paused++
        else if (task.status === 'completed') counts.completed++
        else if (task.status === 'interrupted') counts.interrupted++
        else if (task.status === 'error') counts.error++
    }
    return counts
})

const filteredTasks = computed(() => {
    const tab = activeTab.value
    return taskStore.tasks.filter((task) => {
        if (tab === 'all') return true
        return task.status === tab
    })
})

const pagedTasks = computed(() => {
    const start = (page.value - 1) * pageSize.value
    return filteredTasks.value.slice(start, start + pageSize.value)
})

/** 进行中的任务（清空全部历史时会被取消） */
const ACTIVE_STATUSES: string[] = ['waiting', 'downloading', 'paused', 'processing']
const activeTaskCount = computed(
    () => taskStore.tasks.filter((t) => ACTIVE_STATUSES.includes(t.status)).length
)

/** “全部 / 已完成”标签页提供批量清除入口 */
const inClearableTab = computed(() => activeTab.value === 'all' || activeTab.value === 'completed')
const canClearCompleted = computed(() => inClearableTab.value && tabCounts.value.completed > 0)
const canClearAll = computed(() => inClearableTab.value && tabCounts.value.total > 0)

const showToolbar = computed(
    () =>
        (activeTab.value === 'error' && tabCounts.value.error > 0) ||
        (activeTab.value === 'interrupted' && tabCounts.value.interrupted > 0) ||
        canClearCompleted.value ||
        canClearAll.value
)

// 切换标签页时回到第一页
watch(activeTab, () => {
    page.value = 1
})

// 任务被删除或筛选结果变少时，纠正越界页码，避免停留在空白页
watch(
    () => filteredTasks.value.length,
    (len) => {
        const maxPage = Math.max(1, Math.ceil(len / pageSize.value))
        if (page.value > maxPage) page.value = maxPage
    }
)

async function handleAction(action: TaskAction, taskId: string, extra?: TaskActionExtra) {
    try {
        // 操作统一交给 Rust 命令；列表变化由 task-updated/task-removed 事件回填。
        switch (action) {
            case 'cancel':
                await taskStore.cancelTask(taskId, extra?.deleteFile === true)
                break
            case 'pause':
                await taskStore.pauseTask(taskId)
                break
            case 'resume':
                // 中断任务恢复会重新读取当前下载设置；先完成待写入的设置变更。
                if (taskStore.tasks.find(task => task.id === taskId)?.status === 'interrupted') {
                    await settingsStore.flushSettings()
                }
                await taskStore.resumeTask(taskId)
                break
            case 'retry':
                await retryTask(taskId)
                break
            case 'remove': {
                const result = await taskStore.removeTask(taskId, extra?.deleteFile === true)
                if (result.failed) {
                    throw new Error(result.errors.join('；'))
                }
                break
            }
            case 'open-location': {
                const task = taskStore.tasks.find((t) => t.id === taskId)
                if (task?.filePath) {
                    try {
                        await openFileLocation(task.filePath)
                    } catch (e) {
                        console.error('打开文件位置失败:', e)
                    }
                }
                break
            }
        }
        // 命令成功后才清除选中状态；失败时保留以便用户重试。
        selectedRowKeys.value = selectedRowKeys.value.filter((id) => id !== taskId)
    } catch (e: any) {
        notification.error({
            title: '操作失败',
            description: e?.message || String(e),
            duration: 4000,
        })
    }
}

async function handleBatchClear(deleteFile: boolean) {
    const ids = selectedRowKeys.value.slice()
    if (ids.length === 0) {
        return
    }
    try {
        // 批量命令由后端逐个删除并返回准确计数，前端不直接修改持久化记录。
        const result = await taskStore.removeTasks(ids, deleteFile)
        if (result.failed > 0) {
            notification.warning({
                title: '部分任务未清除',
                description: result.errors.join('；').slice(0, 200),
                duration: 4000,
            })
        }
        // 在批量删除流程完成后再清空选中键，避免删除过程中选中状态提前丢失。
        selectedRowKeys.value = []
    } catch (e: any) {
        notification.error({
            title: '清除任务失败',
            description: e?.message || String(e),
            duration: 4000,
        })
    }
}

/** 仅处理当前标签的任务；中断任务由用户明确发起恢复。 */
async function handleRetryAll() {
    if (retryingAll.value) return
    const targetStatus = activeTab.value === 'interrupted' ? 'interrupted' : 'error'
    const ids = taskStore.tasks
        .filter((task) => task.status === targetStatus)
        .map((t) => t.id)
    if (ids.length === 0) return

    retryingAll.value = true
    try {
        // 批量重试读取同一份当前设置，先完成防抖写盘。
        await settingsStore.flushSettings()
        const { succeeded, failed } = targetStatus === 'interrupted'
            ? await taskStore.resumeTasks(ids)
            : await taskStore.retryTasks(ids)
        notification.success({
            title: targetStatus === 'interrupted' ? '批量恢复' : '批量重试',
            description: `已重新入队 ${succeeded} 个任务${failed > 0 ? `，${failed} 个未能入队，请查看任务状态` : ''}`,
            duration: 4000,
        })
    } catch (e: any) {
        console.error('批量重试失败:', e)
        notification.error({ title: '批量重试失败', description: e?.message || String(e), duration: 4000 })
    } finally {
        retryingAll.value = false
    }
}

/** 清除所有“已下载（已完成）”的任务记录：仅移除记录，不删除磁盘文件。 */
function confirmClearCompleted() {
    const count = tabCounts.value.completed
    if (count === 0) return
    dialog.warning({
        title: '清除已下载任务记录',
        content: `确定清除 ${count} 个已下载的任务记录吗？此操作只移除任务记录，不会修改或删除磁盘上已下载的文件。`,
        positiveText: '确定清除',
        negativeText: '取消',
        onPositiveClick: () => performClearCompleted(),
    })
}

async function performClearCompleted() {
    if (clearing.value) return
    const ids = taskStore.tasks.filter((t) => t.status === 'completed').map((t) => t.id)
    if (ids.length === 0) return

    selectedRowKeys.value = []
    clearing.value = true
    try {
        // 始终只清除记录，不删除文件。
        const result = await taskStore.removeTasks(ids, false)
        const failedMessage = result.failed > 0 ? `，${result.failed} 个失败` : ''
        notification.success({
            title: '已清除',
            description: `已清除 ${result.succeeded} 个已下载的任务记录（未删除文件）${failedMessage}`,
            duration: 4000,
        })
    } finally {
        clearing.value = false
    }
}

/** 清除所有历史任务（含等待/下载/暂停/处理中的任务，会被一并取消）。 */
function confirmClearAll() {
    const total = tabCounts.value.total
    if (total === 0) return
    const activeNote = activeTaskCount.value > 0
        ? `其中 ${activeTaskCount.value} 个任务正在进行（等待/下载/暂停/处理中），会被一并取消。`
        : ''
    dialog.warning({
        title: '清除所有历史任务',
        content: `确定清除全部 ${total} 个任务记录吗？${activeNote}此操作只移除任务记录，不会修改或删除磁盘上已下载的文件。`,
        positiveText: '确定清除',
        negativeText: '取消',
        onPositiveClick: () => performClearAll(),
    })
}

async function performClearAll() {
    if (clearing.value) return
    const ids = taskStore.tasks.map((t) => t.id)
    if (ids.length === 0) return

    selectedRowKeys.value = []
    clearing.value = true
    try {
        // 始终只清除记录，不删除文件。
        const result = await taskStore.removeTasks(ids, false)
        const failedMessage = result.failed > 0 ? `，${result.failed} 个失败` : ''
        notification.success({
            title: '已清除',
            description: `已清除全部 ${result.succeeded} 个任务记录（未删除文件）${failedMessage}`,
            duration: 4000,
        })
    } finally {
        clearing.value = false
    }
}
</script>

<style scoped>
.task-view {
    display: flex;
    flex-direction: column;
    min-height: 100%;
    min-width: 0;
}

.task-pagination {
    display: flex;
    justify-content: center;
    padding: 16px 0 0;
    flex-shrink: 0;
}

.task-toolbar {
    display: flex;
    align-items: center;
    gap: 8px 12px;
    padding: 14px 16px;
    margin-bottom: 16px;
    flex-wrap: wrap;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
}

/* 任务表格放入卡片，隐藏 Naive 表格自带外边框，匹配 MusicDock 版式 */
.table-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
}

.table-card :deep(.n-data-table) {
    --n-border-color: transparent;
}

.table-card :deep(.n-data-table .n-data-table-th) {
    background-color: #fafbfc;
    color: var(--text-tertiary);
    font-weight: 500;
}

.table-card :deep(.n-data-table .n-data-table-td) {
    border-bottom: 1px solid var(--border);
}

.task-toolbar-hint {
    font-size: 12px;
    color: var(--color-text-secondary);
    overflow-wrap: anywhere;
}

.task-toolbar-confirm {
    max-width: 320px;
}

.task-toolbar-warn {
    font-size: 12px;
    color: var(--n-warning-color, #f0a020);
}

@media (max-width: 767px) {
    .task-toolbar {
        align-items: stretch;
    }

    .task-toolbar > .n-button {
        min-height: 44px;
        flex: 1 1 auto;
    }

    .task-toolbar-hint {
        flex-basis: 100%;
    }
}
</style>
