import { h, ref } from 'vue'
import { useDialog, useNotification, NButton } from 'naive-ui'
import type { Quality, SongInfo, QualityItem, DuplicateAction } from '../types'
import { useSettingsStore } from '../stores/settingsStore'
import { useTaskStore } from '../stores/taskStore'
import { useLogStore } from '../stores/logStore'
import QualitySelector from '../components/search/QualitySelector.vue'
import { getQishuiSongDetail } from '../api/musicApi'

/** 只处理用户交互：音质选择、重名确认及通知。品质降级、路径与重试由 Rust 决定。 */
export function useDownloadActions() {
    const dialog = useDialog()
    const notification = useNotification()
    const settingsStore = useSettingsStore()
    const taskStore = useTaskStore()
    const logStore = useLogStore()

    /** 只收集用户选择的品质标签，不在页面上做降级或文件名推断。 */
    function askQuality(qualities: QualityItem[]): Promise<string> {
        return new Promise((resolve, reject) => {
            const compRef = ref<InstanceType<typeof QualitySelector>>()
            const d = dialog.create({
                title: '选择下载音质',
                content: () => h(QualitySelector, { qualities, ref: compRef }),
                positiveText: '确定',
                negativeText: '取消',
                onPositiveClick: () => {
                    const val = compRef.value?.selected
                    if (val) {
                        resolve(val)
                    } else {
                        reject(new Error('未选择品质'))
                    }
                    d.destroy()
                },
                onNegativeClick: () => {
                    reject(new Error('用户取消'))
                    d.destroy()
                },
                onClose: () => reject(new Error('用户取消')),
            })
        })
    }

    /** Rust 返回重名确认请求后，再向用户询问覆盖、保留两份或取消。 */
    function askDuplicateAction(songTitle: string): Promise<DuplicateAction> {
        return new Promise(resolve => {
            const d = dialog.create({
                title: '文件已存在',
                content: `歌曲“${songTitle}”在下载目录中已存在同名文件，请选择处理方式：`,
                action: () => [
                    h(NButton, {
                        size: 'small',
                        onClick: () => {
                            resolve('overwrite')
                            d.destroy()
                        },
                    }, { default: () => '覆盖' }),
                    h(NButton, {
                        size: 'small',
                        type: 'primary',
                        onClick: () => {
                            resolve('rename')
                            d.destroy()
                        },
                    }, { default: () => '保留两份' }),
                    h(NButton, {
                        size: 'small',
                        type: 'error',
                        onClick: () => {
                            resolve('cancel')
                            d.destroy()
                        },
                    }, { default: () => '取消' }),
                ],
                onClose: () => resolve('cancel'),
            })
        })
    }

    async function submitSong(song: SongInfo, quality: string): Promise<'created' | 'error' | 'cancelled'> {
        let result = await taskStore.createTask(song, quality)
        if (result.outcome === 'needs_confirmation') {
            // Rust 发现实际路径冲突后才弹窗；选择结果交回同一命令重新校验。
            const choice = await askDuplicateAction(result.song_title)
            result = await taskStore.createTask(song, quality, choice)
        }
        if (result.outcome === 'cancelled') {
            return 'cancelled'
        }
        if (result.outcome === 'needs_confirmation') {
            throw new Error('文件状态已变化，请重新添加任务')
        }
        if (result.task.status === 'error') {
            return 'error'
        }
        return 'created'
    }

    /**
     * 汽水搜索结果只带一个占位档位（128kaac），真档位要问详情接口。
     * 取不到（没签名/网络问题）就沿用原对象，绝不因为补齐失败而挡住下载。
     */
    async function withQishuiQualities(song: SongInfo): Promise<SongInfo> {
        if (song.platform !== 'qishui') {
            return song
        }
        try {
            const detail = await getQishuiSongDetail(song.mid)
            if (detail.qualities?.length) {
                return {
                    ...song,
                    qualities: detail.qualities,
                    preview: detail.preview ?? song.preview,
                    coverUrl: song.coverUrl || detail.coverUrl,
                }
            }
        } catch (error) {
            console.warn('汽水详情补齐失败，使用搜索结果音质:', error)
        }
        return song
    }

    async function downloadSingle(song: SongInfo, forceQuality?: Quality): Promise<void> {
        try {
            // 需要用户选音质时先补齐档位，选择器里才能看到 spatial/hi_res。
            const target = await withQishuiQualities(song)
            let quality = forceQuality ?? settingsStore.settings.defaultQuality
            if (quality === 'ask') {
                try {
                    quality = await askQuality(target.qualities)
                } catch {
                    return
                }
            }
            // 设置有防抖写盘；先刷入存储，保证 Rust 读取的是用户当前选择。
            await settingsStore.flushSettings()
            const outcome = await submitSong(target, quality)
            if (outcome === 'created') {
                logStore.push({
                    level: 'info',
                    scope: 'download',
                    title: `已加入队列 · ${song.title}`,
                    detail: `${song.artist || '未知歌手'} · ${quality || '自动音质'}`,
                    platform: song.platform,
                })
            }
            if (outcome === 'error') {
                logStore.push({
                    level: 'warn',
                    scope: 'download',
                    title: `音质不可用 · ${song.title}`,
                    detail: `所选音质（${quality || '自动'}）没有可用资源，任务已标记为错误`,
                    platform: song.platform,
                })
                notification.warning({ title: '下载提示', description: `歌曲“${song.title}”所选音质不可用，已标记为错误` })
            }
        } catch (error: any) {
            console.error('下载失败:', error)
            logStore.push({
                level: 'error',
                scope: 'download',
                title: `下载请求失败 · ${song.title}`,
                detail: error?.message || String(error),
                platform: song.platform,
            })
            notification.error({ title: '下载失败', description: error?.message || String(error) })
        }
    }

    async function batchDownload(songs: SongInfo[]): Promise<void> {
        try {
            let quality = settingsStore.settings.defaultQuality
            if (quality === 'ask') {
                // 批量操作只询问一次用户偏好的品质；每首歌是否可用由 Rust 逐一判断。
                const union = new Map<string, QualityItem>()
                for (const song of songs) {
                    for (const item of song.qualities) {
                        union.set(item.quality, item)
                    }
                }
                if (union.size === 0) {
                    quality = ''
                } else {
                    try {
                        quality = await askQuality([...union.values()])
                    } catch {
                        return
                    }
                }
            }
            await settingsStore.flushSettings()
            let errors = 0
            for (const song of songs) {
                // 批量下载不逐首补详情：汽水会一首一发签名请求，批量场景交给 Rust 按可用档位降级。
                const outcome = await submitSong(song, quality)
                if (outcome === 'error') {
                    errors++
                }
            }
            logStore.push({
                level: errors > 0 ? 'warn' : 'info',
                scope: 'download',
                title: `批量加入队列 · ${songs.length - errors} 首`,
                detail: `${quality || '自动音质'}${errors > 0 ? `，${errors} 首无可用音质` : ''}`,
            })
            if (errors > 0) {
                notification.warning({ title: '批量下载', description: `${errors} 首歌曲无可用音质，已标记为错误` })
            }
        } catch (error: any) {
            console.error('批量下载失败:', error)
            logStore.push({
                level: 'error',
                scope: 'download',
                title: '批量下载失败',
                detail: error?.message || String(error),
            })
            notification.error({ title: '批量下载失败', description: error?.message || String(error) })
        }
    }

    async function retryTask(taskId: string): Promise<void> {
        try {
            await settingsStore.flushSettings()
            // 错误重试由 Rust 决定计数、续传和降级；中断任务使用独立的 resume 命令。
            if (!(await taskStore.retryTask(taskId))) {
                logStore.push({
                    level: 'warn',
                    scope: 'download',
                    title: `重试未入队 · ${taskStore.tasks.find((task) => task.id === taskId)?.songTitle ?? taskId}`,
                    detail: '当前任务状态无法重新入队，请查看任务列表',
                })
                notification.warning({ title: '重试失败', description: '任务未重新入队，请查看任务状态' })
            }
        } catch (error: any) {
            logStore.push({
                level: 'error',
                scope: 'download',
                title: '重试失败',
                detail: error?.message || String(error),
            })
            notification.error({ title: '重试失败', description: error?.message || String(error) })
        }
    }

    return { downloadSingle, batchDownload, retryTask }
}
