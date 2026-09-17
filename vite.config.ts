import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'
import { readFileSync } from 'fs'

// 读取 package.json 中的版本号注入到前端
const pkg = JSON.parse(
  readFileSync(path.resolve(import.meta.dirname, './package.json'), 'utf-8')
) as { version: string }

// Rust API 服务地址（本地开发时由 vite 代理 /api 过去）
const proxyTarget = process.env.VITE_PROXY_TARGET ?? 'http://localhost:8080'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    // 设置路径别名，让 import 更简洁（比如 import '@/utils'）
    alias: {
      '@': path.resolve(import.meta.dirname, './src'),
      '@components': path.resolve(import.meta.dirname, './src/components'),
    },
    // 导入时省略的扩展名（默认已支持 .js, .ts, .jsx, .tsx, .json）
    extensions: ['.mjs', '.js', '.ts', '.jsx', '.tsx', '.json', '.vue'],
  },
  clearScreen: false,
  server: {
    watch: {
      // 忽略编辑器/工具链“原子写”产生的临时文件与临时目录
      ignored: ['**/server/**', '**/libs/**', '**/.*.tmpdir/**', '**/*.tmp'],
    },
    hmr: {
      overlay: true, // 报错时是否在浏览器遮罩层显示
    },
    // 把 /api 代理到 Rust API 服务（含 SSE 事件流）
    proxy: {
      '/api': {
        target: proxyTarget,
        changeOrigin: true,
      },
    },
  },
  build: {
    sourcemap: false,
  },
  define: {
    // 把版本号注入环境变量
    'import.meta.env.VITE_APP_VERSION': JSON.stringify(pkg.version),
  },
})
