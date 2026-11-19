<template>
    <div class="access-page">
        <div class="access-card">
            <div class="access-brand">
                <div class="brand-logo">H</div>
                <div>
                    <div class="brand-name">HotDownloader</div>
                    <div class="brand-sub">自托管音乐下载</div>
                </div>
            </div>

            <template v-if="webSession.mode === 'password'">
                <div class="eyebrow access-eyebrow">Server access · 服务访问</div>
                <div class="access-title">连接下载服务</div>
                <p class="access-desc">请输入服务部署时配置的用户名和密码。</p>
                <form @submit.prevent="submit">
                    <n-input v-model:value="username" name="username" size="large" placeholder="用户名"
                        autocomplete="username" />
                    <n-input v-model:value="password" type="password" show-password-on="click" size="large"
                        name="password" placeholder="密码" autocomplete="current-password" class="password-input" />
                    <n-button type="primary" :loading="webSession.checking" class="access-button" attr-type="submit">
                        登录
                    </n-button>
                </form>
            </template>

            <template v-else-if="webSession.mode === 'token'">
                <div class="eyebrow access-eyebrow">Server access · 服务访问</div>
                <div class="access-title">连接下载服务</div>
                <p class="access-desc">请输入服务部署时配置的访问令牌。</p>
                <form @submit.prevent="submit">
                    <n-input v-model:value="token" type="password" show-password-on="click" size="large"
                        placeholder="访问令牌" autocomplete="off" />
                    <n-button type="primary" :loading="webSession.checking" class="access-button" attr-type="submit">
                        连接
                    </n-button>
                </form>
            </template>

            <template v-else>
                <div class="access-title">连接下载服务</div>
                <p class="access-desc">正在连接下载服务…</p>
                <n-button v-if="webSession.error" size="large" class="access-button" :loading="webSession.checking"
                    @click="authorizeWeb()">
                    重试
                </n-button>
            </template>

            <n-alert v-if="webSession.error" type="error" class="access-error">
                {{ webSession.error }}
            </n-alert>

            <p v-if="webSession.mode !== null" class="access-note">
                凭据只保存在当前浏览器会话，关闭页面后需重新输入。
            </p>
        </div>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NAlert, NButton, NInput } from 'naive-ui'
import { authorizeWeb, authorizeWebWithPassword, webSession } from '../api/webClient'

const token = ref('')
const username = ref('')
const password = ref('')

async function submit() {
    if (webSession.mode === 'password') {
        await authorizeWebWithPassword(username.value, password.value)
        password.value = ''
    } else {
        await authorizeWeb(token.value)
    }
}
</script>

<style scoped>
.access-page {
    min-height: 100vh;
    min-height: 100dvh;
    display: grid;
    place-items: center;
    padding: 24px;
    background:
        radial-gradient(900px 480px at 20% -10%, #ffffff 0%, transparent 62%),
        radial-gradient(700px 420px at 105% 105%, #e9e7dc 0%, transparent 58%),
        var(--bg);
}

.access-card {
    width: min(420px, 100%);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    padding: 34px;
    animation: accessIn 0.42s var(--ease-out);
}

@keyframes accessIn {
    from {
        opacity: 0;
        transform: translateY(12px);
    }

    to {
        opacity: 1;
        transform: none;
    }
}

.access-brand {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 30px;
}

.brand-logo {
    width: 38px;
    height: 38px;
    flex: none;
    border-radius: 11px;
    background: var(--accent);
    color: #fff;
    display: grid;
    place-items: center;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 16px;
    letter-spacing: 0.01em;
}

.brand-name {
    font-weight: 600;
    font-size: 14px;
    line-height: 1.2;
    letter-spacing: -0.01em;
    color: var(--text-primary);
}

.brand-sub {
    font-family: var(--mono);
    font-size: 9.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    margin-top: 4px;
}

.access-eyebrow {
    margin-bottom: 8px;
}

.access-title {
    font-size: 24px;
    font-weight: 600;
    letter-spacing: -0.025em;
    color: var(--text-primary);
}

.access-desc {
    margin: 8px 0 22px;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.65;
}

.password-input {
    margin-top: 12px;
}

.access-button {
    width: 100%;
    height: 44px;
    margin-top: 18px;
    font-size: 14px;
    border-radius: 999px !important;
}

.access-error {
    margin-top: 14px;
}

.access-note {
    margin-top: 16px;
    font-size: 11.5px;
    line-height: 1.6;
    color: var(--text-tertiary);
    text-align: center;
}

@media (prefers-reduced-motion: reduce) {
    .access-card {
        animation: none;
    }
}
</style>
