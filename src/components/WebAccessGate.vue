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
        radial-gradient(1200px 600px at 15% -10%, #e8f0fe 0%, transparent 55%),
        radial-gradient(900px 520px at 110% 0%, #eef1ff 0%, transparent 50%),
        var(--bg);
}

.access-card {
    width: min(420px, 100%);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 16px;
    box-shadow: var(--shadow-lg);
    padding: 32px;
    animation: accessIn 0.24s ease;
}

@keyframes accessIn {
    from {
        opacity: 0;
        transform: translateY(8px);
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
    margin-bottom: 26px;
}

.brand-logo {
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 12px;
    background: var(--accent);
    color: #fff;
    display: grid;
    place-items: center;
    font-weight: 700;
    font-size: 18px;
    letter-spacing: 0.5px;
}

.brand-name {
    font-weight: 700;
    font-size: 16px;
    line-height: 1.2;
    color: var(--text-primary);
}

.brand-sub {
    font-size: 12px;
    color: var(--text-tertiary);
    margin-top: 3px;
}

.access-title {
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
}

.access-desc {
    margin: 6px 0 20px;
    color: var(--text-secondary);
    font-size: 13.5px;
    line-height: 1.6;
}

.password-input {
    margin-top: 12px;
}

.access-button {
    width: 100%;
    height: 44px;
    margin-top: 18px;
    font-size: 15px;
}

.access-error {
    margin-top: 14px;
}

@media (prefers-reduced-motion: reduce) {
    .access-card {
        animation: none;
    }
}
</style>
