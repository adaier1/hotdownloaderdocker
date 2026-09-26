<template>
    <div class="search-bar">
        <!-- 平台选择下拉 -->
        <n-dropdown :options="platformDropdownOptions" trigger="click" @select="handlePlatformSelect">
            <button type="button" class="platform-btn">
                <span class="platform-label">{{ currentPlatformLabel }}</span>
                <svg class="platform-arrow" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"
                    stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <polyline points="6 9 12 15 18 9" />
                </svg>
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
.search-bar {
    --search-height: 44px;
    --search-font-size: 14px;

    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    margin-bottom: 24px;
    background: var(--surface);
    padding: 6px 6px 6px 18px;
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-md);
    transition: box-shadow var(--transition);
}

.search-bar:focus-within {
    box-shadow: var(--shadow-lg);
}

/* 平台按钮样式：纯文字 + 下拉箭头 */
.platform-btn {
    height: var(--search-height);
    padding: 0 8px;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    font-family: inherit;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    border: none;
    background: none;
    border-radius: 6px;
    cursor: pointer;
    transition: background var(--transition);
}

.platform-btn:hover {
    background: var(--bg);
}

.platform-arrow {
    width: 12px;
    height: 12px;
    color: var(--text-secondary);
}

/* 输入框容器自动撑开 */
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
    padding: 0 !important;
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
    color: var(--text-tertiary) !important;
    opacity: 1;
}

.search-input :deep(.n-input__border),
.search-input :deep(.n-input__state-border) {
    display: none !important;
}

.search-input :deep(.n-input__suffix) {
    align-items: center;
}

/* 搜索按钮：与 mockup 一致的小圆角主色按钮 */
.search-bar .search-btn {
    height: var(--search-height) !important;
    padding: 0 20px !important;
    font-size: var(--search-font-size) !important;
    border-radius: 10px !important;
    display: inline-flex !important;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}
</style>
