import { defineStore } from 'pinia'
import { ref } from 'vue'
import { isNativeRuntime } from '../api/runtimeApi'
import { taskTransport } from '../api/taskTransport'
import { useSettingsStore } from './settingsStore'
import { useLogStore } from './logStore'
import type { BatchTaskResult } from '../api/taskTransport'
import type {
    SongInfo,
    TaskRecord,
    CreateTaskRequest,
    DuplicateAction,
} from '../types'

/** Rust 持有任务和状态流转；此 store 仅保存页面展示用的投影。 */
export const useTaskStore = defineStore('tasks', () => {
    const logStore = useLogStore()
    const tasks = ref<TaskRecord[]>([])
    const connectionStatus = ref<'connecting' | 'connected' | 'reconnecting' | 'disconnected'>(
        isNativeRuntime() ? 'connected' : 'connecting'
    )
    const lastServerActivityAt = ref<number | null>(null)
    let loading = false
    let pendingEvents: Array<() => void> = []

    function projectEvent(apply: () => void) {
        // 加载快照期间收到的事件必须排队，快照落地后再重放，避免覆盖较新的状态。
        if (loading) {
            pendingEvents.push(apply)
        } else {
            apply()
        }
    }

    function upsert(task: TaskRecord) {
        const index = tasks.value.findIndex(t => t.id === task.id)
        if (index < 0) {
            tasks.value.push(task)
        } else {
            tasks.value[index] = task
        }
    }

    async function loadTasks() {
        loading = true
        try {
            tasks.value = await taskTransport.list()
        } finally {
            loading = false
            // 加载失败时也要重放事件，避免页面永远停留在旧状态。
            const queued = pendingEvents
            pendingEvents = []
            queued.forEach(apply => apply())
        }
    }

    function createTask(song: SongInfo, desiredQuality: string, duplicateAction?: DuplicateAction) {
        const request: CreateTaskRequest = { song, desiredQuality, duplicateAction }
        // 不在前端预写任务；命令返回及 task-updated 事件都以 Rust 的记录为准。
        return taskTransport.create(request)
    }

    async function cancelTask(taskId: string, deleteFile = false) {
        await taskTransport.cancel(taskId, deleteFile)
    }

    async function removeTasks(taskIds: string[], deleteFile = false): Promise<BatchTaskResult> {
        if (taskIds.length === 0) {
            return { succeeded: 0, failed: 0, errors: [] }
        }
        return taskTransport.remove(taskIds, deleteFile)
    }

    async function removeTask(taskId: string, deleteFile = false) {
        return removeTasks([taskId], deleteFile)
    }

    async function pauseTask(taskId: string) {
        await taskTransport.pause(taskId)
    }

    async function resumeTask(taskId: string) {
        await taskTransport.resume(taskId)
    }

    /** 按顺序请求恢复中断任务；是否可恢复由 Rust 根据任务状态决定。 */
    async function resumeTasks(taskIds: string[]): Promise<{ total: number; succeeded: number; failed: number }> {
        let succeeded = 0
        let failed = 0
        for (const id of taskIds) {
            try {
                await resumeTask(id)
                succeeded++
            } catch (error) {
                console.error('恢复任务失败:', id, error)
                failed++
            }
        }
        return { total: taskIds.length, succeeded, failed }
    }

    function retryTask(taskId: string): Promise<boolean> {
        // 后端统一处理重试次数、断点续传及音质降级；false 表示当前无法继续重试。
        return taskTransport.retry(taskId)
    }

    /** 逐个发起重试命令，实际下载并发仍由 Rust 调度器控制。 */
    async function retryTasks(taskIds: string[]): Promise<{ total: number; succeeded: number; failed: number }> {
        let succeeded = 0
        let failed = 0
        for (const id of taskIds) {
            try {
                if (await retryTask(id)) {
                    succeeded++
                } else {
                    failed++
                }
            } catch (error) {
                console.error('重试任务失败:', id, error)
                failed++
            }
        }
        return { total: taskIds.length, succeeded, failed }
    }

    async function setupListeners(): Promise<() => void> {
        // 从开始订阅到快照加载完成都缓存事件，避免初始化期间的事件被快照覆盖。
        loading = true
        return taskTransport.subscribe({
            connection(status) {
                const previous = connectionStatus.value
                connectionStatus.value = status
                // 只记录真正的状态切换：退订（页面卸载/重新登录）不会产生日志，
                // 断线以 reconnecting 表示，重连过程中的抖动不重复写。
                if (status === 'reconnecting' && previous === 'connected') {
                    logStore.push({
                        level: 'warn',
                        scope: 'system',
                        title: '与服务端的实时连接中断',
                        detail: '事件流已断开，正在尝试重连',
                    })
                } else if (status === 'connected' && previous === 'reconnecting') {
                    logStore.push({
                        level: 'info',
                        scope: 'system',
                        title: '连接已恢复',
                        detail: '实时任务事件重新订阅成功',
                    })
                }
            },
            activity(at) {
                lastServerActivityAt.value = at
            },
            settings(snapshot) {
                useSettingsStore().applyServerSnapshot(snapshot)
            },
            snapshot(snapshot) {
                // SSE 重连会重新发完整快照；由后端记录直接替换只读投影。
                projectEvent(() => { tasks.value = snapshot })
            },
            updated(task) {
                projectEvent(() => {
                    const previous = tasks.value.find(t => t.id === task.id)
                    upsert(task)

                    // 只在状态变化时提醒，下载进度事件不会反复弹通知。
                    if (previous && previous.status !== task.status) {
                        if (task.status === 'completed') {
                            // Web 端由服务端写入日志（同一批任务多标签页只留一条），
                            // 桌面端没有独立服务，仍在页面侧记录。
                            if (isNativeRuntime()) {
                                logStore.push({
                                    level: 'success',
                                    scope: 'download',
                                    title: `下载完成 · ${task.songTitle}`,
                                    detail: `${task.artist || '未知歌手'} · ${task.quality}${task.filePath ? ` · ${task.filePath}` : ''}`,
                                    platform: task.platform,
                                })
                            }
                            window.$notify?.success({
                                title: '下载完成',
                                description: `歌曲“${task.songTitle}”已下载完成`,
                                duration: 3000,
                            })
                        } else if (task.status === 'error') {
                            if (isNativeRuntime()) {
                                logStore.push({
                                    level: 'error',
                                    scope: 'download',
                                    title: `下载失败 · ${task.songTitle}`,
                                    detail: `${task.artist || '未知歌手'} · ${task.errorMsg?.trim() || '未知原因'}`,
                                    platform: task.platform,
                                })
                            }
                            window.$notify?.error({
                                title: '下载失败',
                                description: `歌曲“${task.songTitle}”错误：${(task.errorMsg || '').slice(0, 100)}`,
                                duration: 3000,
                            })
                        }
                    }
                })
            },
            removed(taskId) {
                projectEvent(() => {
                    tasks.value = tasks.value.filter(t => t.id !== taskId)
                })
            },
            // 元数据写入失败不等于文件下载失败，保留独立的提示事件。
            metadataError(error) {
                const task = tasks.value.find(t => t.id === error.task_id)
                // 服务端已把这条写入运行日志，Web 端重复提交会出现两条。
                if (isNativeRuntime()) {
                    logStore.push({
                        level: 'warn',
                        scope: 'download',
                        title: `元数据写入失败 · ${task?.songTitle ?? error.task_id}`,
                        detail: error.error_msg,
                        platform: task?.platform,
                    })
                }
                window.$notify?.warning({
                    title: '元数据写入失败',
                    description: `歌曲“${task?.songTitle ?? error.task_id}”元数据写入失败：${error.error_msg}`,
                    duration: 3000,
                })
            },
            loginRefreshFailed(message) {
                logStore.push({
                    level: 'error',
                    scope: 'account',
                    title: '登录状态刷新失败',
                    detail: `${message}；请到账号页重新登录`,
                })
                window.$notify?.error({
                    title: '登录刷新失败，请重新登录',
                    description: message,
                    duration: 5000,
                })
            },
            // 服务端写入的运行日志（下载完成/失败等）：日志页与其他设备看到同一份流水。
            log(entry) {
                logStore.applyRemote(entry)
            },
            logsCleared() {
                logStore.applyCleared()
            },
        })
    }

    return {
        tasks,
        connectionStatus,
        lastServerActivityAt,
        loadTasks,
        createTask,
        cancelTask,
        removeTasks,
        removeTask,
        pauseTask,
        resumeTask,
        resumeTasks,
        retryTask,
        retryTasks,
        setupListeners,
    }
})
