<template>
    <div class="search-bar md-search">
        <!-- 平台选择：胶囊前缀（对应 MusicDock 的 KM 标记） -->
        <n-dropdown :options="platformDropdownOptions" trigger="click" @select="handlePlatformSelect">
            <button type="button" class="md-kb-chip platform-chip">
                {{ currentPlatformLabel }}<span class="platform-arrow">▾</span>
            </button>
        </n-dropdown>

        <n-input v-model:value="keywordModel" :placeholder="placeholder" clearable @keyup.enter="handleSearch"
            @clear="handleClear" class="search-input" />
        <n-button type="primary" @click="handleSearch" :disabled="!keywordModel.trim() || loading" :loading="loading"
            class="search-btn">
            {{ buttonText }}
        </n-button>
    </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { NInput, NButton, NDropdown } from 'naive-ui'
import type { PlatformOption } from '../../config/platforms'

const props = withDefaults(
    defineProps<{
        keyword: string
        placeholder?: string
        buttonText?: string
        loading?: boolean
        platform: string
        platformOptions: PlatformOption[]
    }>(),
    {
        placeholder: '搜索歌曲、歌手、专辑',
        buttonText: '搜索',
        loading: false,
    }
)

const emit = defineEmits<{
    (e: 'update:keyword', value: string): void
    (e: 'update:platform', value: string): void
    (e: 'search'): void
    (e: 'clear'): void
}>()

const keywordModel = ref(props.keyword)

// 当前平台对应的显示 label
const currentPlatformLabel = computed(() => {
    const found = props.platformOptions.find(p => p.key === props.platform)
    return found ? found.label : props.platform
})

// 下拉选项格式：Naive UI 需要 { label, key } 结构
const platformDropdownOptions = computed(() => {
    return props.platformOptions.map(p => ({
        label: p.label,
        key: p.key,
    }))
})

// 平台选择处理：触发 update:platform 事件，父组件更新 platform 值
function handlePlatformSelect(key: string) {
    emit('update:platform', key)
}

// 向上同步 keyword
watch(keywordModel, (val) => {
    emit('update:keyword', val)
})

// 向下同步 keyword：当父组件 keyword 变化时更新输入框
watch(
    () => props.keyword,
    (newVal) => {
        if (newVal !== keywordModel.value) {
            keywordModel.value = newVal
        }
    }
)

function handleSearch() {
    if (keywordModel.value.trim()) {
        emit('search')
    }
}

function handleClear() {
    // 点击清空按钮时，输入框已经变为空，同时通知父组件清理页面状态
    emit('clear')
}
</script>

<style scoped>
/* 外层版式由 .md-search 提供；这里只负责内部控件的透明与尺寸 */
.search-bar {
    --search-height: 40px;
    --search-font-size: 15px;
    gap: 12px;
}

.platform-chip {
    border: none;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-family: inherit;
}

.platform-arrow {
    font-size: 11px;
    opacity: 0.8;
}

.search-bar .search-input {
    flex: 1;
    min-width: 0;
    background: transparent !important;
    border: none !important;
    box-shadow: none !important;
    height: var(--search-height) !important;
    border-radius: 0 !important;
}

.search-input :deep(.n-input-wrapper) {
    background: transparent !important;
    border: none !important;
    height: 100% !important;
}

.search-input :deep(.n-input__input) {
    padding: 0 !important;
    font-size: var(--search-font-size) !important;
    height: 100% !important;
    line-height: var(--search-height) !important;
    background: transparent !important;
    border: none !important;
    box-shadow: none !important;
    color: inherit !important;
}

.search-input :deep(.n-input__input-el) {
    height: 100%;
}

.search-input :deep(.n-input__input-el::placeholder) {
    color: inherit !important;
    opacity: 0.6;
}

.search-input :deep(.n-input__border),
.search-input :deep(.n-input__state-border) {
    display: none !important;
}

.search-input :deep(.n-input__suffix) {
    align-items: center;
}

.search-bar .search-btn {
    height: var(--search-height) !important;
    padding: 0 22px !important;
    font-size: 14px !important;
    border-radius: 9px !important;
    display: inline-flex !important;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}

@media (max-width: 767px) {
    .search-bar {
        flex-wrap: wrap;
    }

    .search-btn {
        min-height: 44px;
    }
}
</style>
