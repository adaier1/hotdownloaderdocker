<template>
    <div class="library-view">
        <header class="page-head rise" style="--i: 0">
            <div class="page-head-main">
                <div class="eyebrow">Library · 曲库</div>
                <div class="page-title">曲库</div>
                <div class="page-sub">下载目录中已落盘的每一首 · 共 {{ store.files.length }} 首 · {{ formatFileSize(store.totalSize) }}</div>
            </div>
            <div class="page-head-actions">
                <button class="btn-line danger" type="button" :disabled="selected.length === 0" @click="batchDelete">
                    批量删除{{ selected.length > 0 ? `（${selected.length}）` : '' }}
                </button>
                <button class="btn-line" type="button" :disabled="store.loading" @click="reload">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"
                        stroke-linejoin="round" aria-hidden="true">
                        <path d="M21 12a9 9 0 1 1-2.6-6.4M21 3v6h-6" />
                    </svg>
                    {{ store.loading ? '读取中…' : '刷新' }}
                </button>
            </div>
        </header>

        <div class="dir-bar rise" style="--i: 1">
            <span class="dir-label">DOWNLOAD DIR</span>
            <span class="dir-path" :title="store.directory || '未知'">{{ store.directory || '未知' }}</span>
        </div>

        <div v-if="store.error" class="state-error">
            读取曲库失败：{{ store.error }}
        </div>

        <div class="card rise" style="--i: 2">
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

.btn-line.danger {
    color: var(--danger);
    border-color: #e8d2cd;
}

.btn-line.danger:disabled {
    opacity: 0.4;
}

.dir-bar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 16px;
    background: var(--surface-sunken);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
    min-width: 0;
}

.dir-label {
    flex: none;
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.14em;
    color: var(--text-tertiary);
}

.dir-path {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.state-error {
    margin-top: 16px;
    padding: 12px 16px;
    border-radius: var(--radius-md);
    background: var(--danger-weak);
    color: var(--danger);
    font-size: 13px;
}

.card {
    margin-top: 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
}

.toolbar {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 13px 18px;
    border-bottom: 1px solid var(--border);
}

.input {
    flex: 1;
    min-width: 0;
    height: 36px;
    padding: 0 13px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-family: inherit;
    background: var(--surface-sunken);
    color: var(--text-primary);
    transition: border-color var(--transition), box-shadow var(--transition), background var(--transition);
}

.input:focus {
    outline: none;
    border-color: var(--border-2);
    background: var(--surface);
    box-shadow: 0 0 0 4px rgba(28, 28, 22, 0.05);
}

.input::placeholder {
    color: var(--text-tertiary);
}

.count-text {
    flex: none;
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
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
    font-family: var(--mono);
    font-size: 10.5px;
    font-weight: 500;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    text-align: left;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
    background: transparent;
}

td {
    padding: 13px 18px;
    border-bottom: 1px solid var(--border-light);
    font-size: 13.5px;
    vertical-align: middle;
}

th:last-child {
    text-align: right;
}

tbody tr:last-child td {
    border-bottom: none;
}

tbody tr:hover {
    background: var(--surface-hover);
}

.cell-title {
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.cell-sub {
    font-size: 11.5px;
    color: var(--text-tertiary);
    margin-top: 3px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.cell-size {
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
}

.tag {
    display: inline-block;
    font-family: var(--mono);
    font-size: 10px;
    letter-spacing: 0.04em;
    padding: 3px 8px;
    border-radius: 5px;
    white-space: nowrap;
    line-height: 1.55;
    border: 1px solid var(--border-light);
    color: var(--text-secondary);
    background: var(--surface-sunken);
    text-transform: uppercase;
}

.tag.gold {
    color: #86601e;
    border-color: #e6d8b4;
    background: #fbf5e4;
}

.tag.gray {
    color: var(--text-tertiary);
    background: var(--surface-sunken);
    border-color: transparent;
}

.row-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    justify-content: flex-end;
}

.link-action {
    border: none;
    background: none;
    padding: 4px 9px;
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: color var(--transition), background var(--transition);
}

.link-action:hover {
    background: var(--surface-sunken);
    color: var(--text-primary);
}

.btn-del {
    border: 1px solid transparent;
    background: none;
    color: var(--text-tertiary);
    border-radius: var(--radius-pill);
    padding: 4px 9px;
    font-size: 12px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: color var(--transition), background var(--transition);
}

.btn-del:hover {
    color: var(--danger);
    background: var(--danger-weak);
}

.empty-cell {
    text-align: center;
    color: var(--text-tertiary);
    padding: 44px 18px;
    font-size: 13px;
}

@media (max-width: 767px) {
    table {
        table-layout: auto;
    }

    th,
    td {
        padding: 12px;
    }

    .page-head-actions {
        width: 100%;
    }
}
</style>
