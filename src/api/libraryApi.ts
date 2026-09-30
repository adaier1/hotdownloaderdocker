import { invoke, isTauri } from '@tauri-apps/api/core'
import { webRequest } from './webClient'

/** 下载目录中的单个音频文件；字段与 Rust `AudioFileEntry` 对应。 */
export interface AudioFileEntry {
    /** 文件名（含扩展名）。 */
    name: string
    /** 文件绝对路径。 */
    path: string
    /** 文件字节大小。 */
    size: number
    /** 小写扩展名，不含点。 */
    extension: string
}

export interface LibraryResponse {
    /** 实际扫描的下载目录。 */
    directory: string
    files: AudioFileEntry[]
}

export interface DeleteFailure {
    path: string
    error: string
}

export interface DeleteResult {
    deleted: number
    failed: DeleteFailure[]
}

/**
 * 列出下载目录中的音频文件（曲库数据源）。
 *
 * 独立服务由 Rust 决定下载目录；桌面端由调用方传入当前设置中的目录。
 */
export async function fetchLibrary(directory?: string): Promise<LibraryResponse> {
    if (isTauri()) {
        const target = directory ?? ''
        const files = await invoke<AudioFileEntry[]>('list_audio_files', { directory: target })
        return { directory: target, files }
    }
    return webRequest<LibraryResponse>('/api/library')
}

/** 删除下载目录中指定的音频文件。 */
export async function deleteLibraryFiles(paths: string[], directory?: string): Promise<DeleteResult> {
    if (isTauri()) {
        return invoke<DeleteResult>('delete_audio_files', { directory: directory ?? '', paths })
    }
    return webRequest<DeleteResult>('/api/library/delete', {
        method: 'POST',
        body: JSON.stringify({ paths }),
    })
}
