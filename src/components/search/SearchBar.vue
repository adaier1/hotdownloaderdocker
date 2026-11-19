<template>
    <div class="search-bar-group">
        <div class="search-bar md-search">
            <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"
                stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <circle cx="11" cy="11" r="7.5" />
                <path d="m20.5 20.5-4.2-4.2" />
            </svg>
            <n-input v-model:value="keywordModel" :placeholder="placeholder" clearable @keyup.enter="handleSearch"
                @clear="handleClear" class="search-input" />

            <!-- 平台选择：作为检索范围内嵌在输入框右侧 -->
            <div class="platform-switch">
                <button v-for="p in platformOptions" :key="p.key" type="button" class="platform-chip"
                    :class="{ active: p.key === platform }" @click="handlePlatformSelect(p.key)">
                    {{ p.label }}
                </button>
            </div>

            <n-button type="primary" @click="handleSearch" :disabled="!keywordModel.trim() || loading" :loading="loading"
                class="search-btn">
                {{ buttonText }}
            </n-button>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { NInput, NButton } from 'naive-ui'
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
.search-bar-group {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
}

.search-bar {
    --search-height: 52px;
    --search-font-size: 15px;
    gap: 10px;
    padding: 5px 7px 5px 20px;
    max-width: 820px;
}

.search-icon {
    width: 18px;
    height: 18px;
    flex: none;
    color: var(--text-tertiary);
    transition: color var(--transition);
}

.search-bar:focus-within .search-icon {
    color: var(--text-primary);
}

/* 平台切换：内嵌在输入框右侧的等宽小胶囊 */
.platform-switch {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
    padding-left: 10px;
    border-left: 1px solid var(--border-light);
}

.platform-chip {
    height: 26px;
    padding: 0 9px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: transparent;
    color: var(--text-tertiary);
    font-family: var(--mono);
    font-size: 10.5px;
    letter-spacing: 0.08em;
    cursor: pointer;
    transition: color var(--transition), background var(--transition), border-color var(--transition);
}

.platform-chip:hover {
    color: var(--text-primary);
    background: var(--surface-sunken);
}

.platform-chip.active {
    color: var(--surface);
    background: var(--accent);
    border-color: var(--accent);
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

.search-bar .search-btn:disabled {
    --n-color-disabled: var(--surface-sunken);
    --n-text-color-disabled: var(--text-tertiary);
    --n-border-disabled: 1px solid transparent;
}

.search-bar .search-btn {
    height: calc(var(--search-height) - 12px) !important;
    padding: 0 22px !important;
    font-size: 13.5px !important;
    border-radius: 999px !important;
    display: inline-flex !important;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}

@media (max-width: 767px) {
    .search-bar {
        flex-wrap: wrap;
        padding: 8px 10px 10px 16px;
        border-radius: 22px;
    }

    .platform-switch {
        order: 3;
        width: 100%;
        padding-left: 0;
        padding-top: 2px;
        border-left: none;
    }

    .search-btn {
        min-height: 40px;
    }

    .platform-chip {
        min-height: 34px;
    }
}
</style>
