import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { deleteLibraryFiles, fetchLibrary, type AudioFileEntry, type DeleteResult } from '../api/libraryApi'
import { isNativeRuntime } from '../api/runtimeApi'
import { useSettingsStore } from './settingsStore'

/**
 * 曲库数据源：下载目录中的实际音频文件。
 *
 * 仪表盘与曲库页共用该缓存，避免每次进入页面都重复扫描目录。
 */
export const useLibraryStore = defineStore('library', () => {
    const files = ref<AudioFileEntry[]>([])
    const directory = ref('')
    const loading = ref(false)
    const error = ref('')
    let loaded = false

    /** 桌面端下载目录来自设置；网页端由服务端决定，返回空串即可。 */
    async function resolveDirectory(): Promise<string> {
        if (!isNativeRuntime()) return ''
        const settingsStore = useSettingsStore()
        if (settingsStore.settings.downloadDir) return settingsStore.settings.downloadDir
        try {
            await settingsStore.getDefaultDownloadDir()
        } catch {
            /* 目录查询失败时回退到空串，由后端报错 */
        }
        return settingsStore.settings.downloadDir ?? ''
    }

    async function load(force = false) {
        if (loading.value || (loaded && !force)) return
        loading.value = true
        error.value = ''
        try {
            const target = await resolveDirectory()
            const result = await fetchLibrary(target)
            files.value = result.files
            directory.value = result.directory || target
            loaded = true
        } catch (e) {
            error.value = e instanceof Error ? e.message : String(e)
            files.value = []
        } finally {
            loading.value = false
        }
    }

    const totalSize = computed(() => files.value.reduce((sum, file) => sum + file.size, 0))

    /** 删除指定文件并刷新列表。 */
    async function remove(paths: string[]): Promise<DeleteResult> {
        if (paths.length === 0) return { deleted: 0, failed: [] }
        const result = await deleteLibraryFiles(paths, directory.value)
        await load(true)
        return result
    }

    return { files, directory, loading, error, totalSize, load, remove }
})
