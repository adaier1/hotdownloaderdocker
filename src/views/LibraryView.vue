<template>
    <div class="library-view">
        <div class="head-row">
            <div>
                <div class="page-title">曲库</div>
                <div class="page-sub">下载目录中的歌曲 · 共 {{ store.files.length }} 首</div>
            </div>
            <div class="head-right">
                <button class="md-btn-dl batch-btn" type="button" :disabled="selected.length === 0"
                    @click="batchDelete">
                    批量删除{{ selected.length > 0 ? `（${selected.length}）` : '' }}
                </button>
                <button class="btn ghost sm" type="button" :disabled="store.loading" @click="reload">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"
                        stroke-linejoin="round">
                        <path d="M21 12a9 9 0 1 1-2.6-6.4M21 3v6h-6" />
                    </svg>
                    {{ store.loading ? '读取中…' : '刷新' }}
                </button>
            </div>
        </div>

        <div class="dir-bar">
            <span class="dir-label">下载目录</span>
            <span class="dir-path" :title="store.directory || '未知'">{{ store.directory || '未知' }}</span>
        </div>

        <div v-if="store.error" class="state-error">
            读取曲库失败：{{ store.error }}
        </div>

        <div class="card">
            <div class="toolbar">
                <n-checkbox :checked="allSelected" :indeterminate="someSelected" @update:checked="toggleAll">
                    全选
                </n-checkbox>
                <input v-model="keyword" class="input" type="text" placeholder="在曲库中筛选歌曲名或歌手" />
                <span class="count-text">{{ filtered.length }} / {{ store.files.length }}</span>
            </div>

            <div class="table-wrap">
                <table>
                    <colgroup>
                        <col style="width: 44px" />
                        <col style="width: 55%" />
                        <col style="width: 12%" />
                        <col style="width: 16%" />
                        <col style="width: 17%" />
                    </colgroup>
                    <thead>
                        <tr>
                            <th></th>
                            <th>歌曲信息</th>
                            <th>音质</th>
                            <th>文件大小</th>
                            <th>操作</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr v-for="file in filtered" :key="file.path">
                            <td>
                                <n-checkbox :checked="isSelected(file.path)"
                                    @update:checked="(val) => toggleSelect(file.path, val)" />
                            </td>
                            <td>
                                <div class="cell-title">{{ songOf(file).title }}</div>
                                <div class="cell-sub">{{ songOf(file).artist }}</div>
                            </td>
                            <td>
                                <span class="tag" :class="songOf(file).qualityClass">{{ songOf(file).quality }}</span>
                            </td>
                            <td class="cell-size">{{ formatFileSize(file.size) }}</td>
                            <td>
                                <div class="row-actions">
                                    <button v-if="native" class="link-action" type="button"
                                        @click="openLocation(file)">打开位置</button>
                                    <button class="btn-del" type="button" @click="deleteOne(file)">删除</button>
                                </div>
                            </td>
                        </tr>
                        <tr v-if="!store.loading && filtered.length === 0">
                            <td colspan="5" class="empty-cell">
                                {{ store.files.length === 0 ? '曲库为空，去搜索或解析歌单下载音乐吧' : '没有匹配的歌曲' }}
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed, onActivated, onMounted, ref, watch } from 'vue'
import { NCheckbox, useDialog } from 'naive-ui'
import { useLibraryStore } from '../stores/libraryStore'
import { isNativeRuntime } from '../api/runtimeApi'
import { openFileLocation } from '../api/fileApi'
import { formatFileSize } from '../utils/format'
import type { AudioFileEntry } from '../api/libraryApi'

const store = useLibraryStore()
const dialog = useDialog()
const native = isNativeRuntime()
const keyword = ref('')
const selected = ref<string[]>([])

const filtered = computed(() => {
    const query = keyword.value.trim().toLowerCase()
    if (!query) return store.files
    return store.files.filter((file) => file.name.toLowerCase().includes(query))
})

const allSelected = computed(
    () => filtered.value.length > 0 && filtered.value.every((file) => selected.value.includes(file.path))
)

const someSelected = computed(
    () => selected.value.length > 0 && !allSelected.value
)

/** 音质标签由扩展名推导；母带等高音质沿用金色标签。 */
const GOLD_QUALITIES = new Set(['flac', 'ape', 'wav'])

function songOf(file: AudioFileEntry) {
    const base = file.name.replace(/\.[^.]+$/, '')
    const parts = base.split(' - ')
    const title = parts[0] || base
    const artist = parts.length > 1 ? parts[1] : '未知歌手'
    const quality = (parts.length > 3 ? parts[3] : file.extension).toUpperCase()
    return {
        title,
        artist,
        quality,
        qualityClass: GOLD_QUALITIES.has(file.extension) ? 'gold' : 'gray',
    }
}

function isSelected(path: string): boolean {
    return selected.value.includes(path)
}

function toggleSelect(path: string, checked: boolean) {
    if (checked) {
        if (!selected.value.includes(path)) selected.value = [...selected.value, path]
    } else {
        selected.value = selected.value.filter((item) => item !== path)
    }
}

function toggleAll(checked: boolean) {
    if (checked) {
        const visible = filtered.value.map((file) => file.path)
        selected.value = Array.from(new Set([...selected.value, ...visible]))
    } else {
        const visible = new Set(filtered.value.map((file) => file.path))
        selected.value = selected.value.filter((path) => !visible.has(path))
    }
}

// 列表刷新后清理已不存在的选中项。
watch(() => store.files, (files) => {
    const existing = new Set(files.map((file) => file.path))
    selected.value = selected.value.filter((path) => existing.has(path))
})

async function reload() {
    await store.load(true)
}

async function openLocation(file: AudioFileEntry) {
    try {
        await openFileLocation(file.path)
    } catch (e) {
        console.error('打开文件位置失败:', e)
    }
}

function notify(type: 'success' | 'error', title: string, description: string) {
    window.$notify?.[type]({ title, description, duration: 3000 })
}

async function deleteOne(file: AudioFileEntry) {
    try {
        const result = await store.remove([file.path])
        if (result.deleted > 0) {
            notify('success', '已删除', `已删除「${songOf(file).title}」`)
        } else {
            notify('error', '删除失败', result.failed[0]?.error || '未知原因')
        }
    } catch (e) {
        notify('error', '删除失败', e instanceof Error ? e.message : String(e))
    }
}

function batchDelete() {
    const paths = [...selected.value]
    if (paths.length === 0) return
    dialog.warning({
        title: '批量删除',
        content: `确定删除已勾选的 ${paths.length} 个文件吗？此操作不可恢复。`,
        positiveText: '删除',
        negativeText: '取消',
        onPositiveClick: async () => {
            try {
                const result = await store.remove(paths)
                selected.value = []
                if (result.failed.length > 0) {
                    notify('error', '部分删除失败', `${result.deleted} 个成功，${result.failed.length} 个失败`)
                } else {
                    notify('success', '已删除', `已删除 ${result.deleted} 个文件`)
                }
            } catch (e) {
                notify('error', '删除失败', e instanceof Error ? e.message : String(e))
            }
        },
    })
}

onMounted(() => { void store.load() })
onActivated(() => { void store.load(true) })
</script>

<style scoped>
.library-view {
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
    flex-wrap: wrap;
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
    display: flex;
    align-items: center;
    gap: 10px;
    padding-top: 4px;
}

.batch-btn {
    height: 32px;
    padding: 0 16px;
    font-size: 13px;
    border-radius: 8px;
}

.batch-btn:disabled {
    opacity: 0.5;
    cursor: default;
}

.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 32px;
    padding: 0 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    font-family: inherit;
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-secondary);
    transition: color var(--transition), border-color var(--transition);
}

.btn:hover:not(:disabled) {
    color: var(--text-primary);
    border-color: #c9cdd4;
}

.btn:disabled {
    opacity: 0.6;
    cursor: default;
}

.btn svg {
    width: 15px;
    height: 15px;
}

.dir-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 18px;
    padding: 10px 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    min-width: 0;
}

.dir-label {
    flex: none;
    font-size: 12.5px;
    color: var(--text-tertiary);
}

.dir-path {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    font-size: 13px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.state-error {
    margin-top: 16px;
    padding: 12px 16px;
    border-radius: var(--radius-md);
    background: var(--danger-weak, #fff1f0);
    color: var(--danger);
    font-size: 13px;
}

.card {
    margin-top: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
}

.toolbar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
}

.input {
    flex: 1;
    min-width: 0;
    height: 38px;
    padding: 0 14px;
    border: 1px solid var(--border);
    border-radius: 9px;
    font-size: 14px;
    font-family: inherit;
    background: #fff;
    color: var(--text-primary);
    transition: border-color var(--transition), box-shadow var(--transition);
}

.input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(43, 107, 243, 0.12);
}

.count-text {
    flex: none;
    font-size: 12.5px;
    color: var(--text-tertiary);
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
    background: #fafbfc;
}

td {
    padding: 16px 18px;
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

.cell-size {
    color: var(--text-secondary);
}

.tag {
    display: inline-block;
    font-size: 12px;
    padding: 3px 10px;
    border-radius: 6px;
    white-space: nowrap;
    line-height: 1.5;
    border: 1px solid var(--border-2, #dde1e6);
    color: var(--text-secondary);
    background: #fff;
}

.tag.gold {
    color: #8a5a00;
    border-color: #e9cd7c;
    background: #fff8e3;
}

.tag.gray {
    color: var(--text-tertiary);
    background: #f4f5f7;
    border-color: transparent;
}

.row-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
}

.link-action {
    border: none;
    background: none;
    padding: 4px 8px;
    border-radius: 6px;
    color: var(--accent);
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: background var(--transition);
}

.link-action:hover {
    background: var(--accent-light);
}

.btn-del {
    border: none;
    background: var(--danger);
    color: #fff;
    border-radius: 8px;
    padding: 7px 18px;
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: background var(--transition);
}

.btn-del:hover {
    background: #e63a3c;
}

.empty-cell {
    text-align: center;
    color: var(--text-tertiary);
    padding: 44px 18px;
}

@media (max-width: 767px) {
    table {
        table-layout: auto;
    }

    th,
    td {
        padding: 12px;
    }

    .head-right {
        width: 100%;
    }
}
</style>
