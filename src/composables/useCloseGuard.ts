import { onMounted, onUnmounted } from 'vue'
import { useTaskStore } from '../stores/taskStore'

/**
 * Web 版关闭守卫：浏览器无法拦截关闭窗口（服务端任务独立于浏览器运行），
 * 这里在存在未完成任务时通过 `beforeunload` 提示用户。
 */
export function useCloseGuard() {
    const taskStore = useTaskStore()

    const handler = (event: BeforeUnloadEvent) => {
        const activeTasks = taskStore.tasks.filter(
            (t) =>
                t.status === 'waiting' ||
                t.status === 'downloading' ||
                t.status === 'paused'
        )
        if (activeTasks.length === 0) return

        // 服务端任务不会因关闭页面而中断，仅提示用户
        event.preventDefault()
        event.returnValue = ''
    }

    onMounted(() => {
        window.addEventListener('beforeunload', handler)
    })

    onUnmounted(() => {
        window.removeEventListener('beforeunload', handler)
    })
}
