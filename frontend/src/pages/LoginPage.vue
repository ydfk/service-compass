<script setup lang="ts">
import { ArrowLeft, Key } from '@vicons/tabler'
import { NButton, NCard, NForm, NFormItem, NIcon, NInput, useMessage } from 'naive-ui'
import { ref } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const username = ref('')
const password = ref('')
const loading = ref(false)
const auth = useAuthStore()
const router = useRouter()
const route = useRoute()
const message = useMessage()

async function submit() {
  if (!username.value || !password.value) return
  loading.value = true
  try {
    await auth.login(username.value, password.value)
    await router.replace((route.query.redirect as string) || '/admin')
  } catch (error) {
    message.error(error instanceof Error ? error.message : '登录失败')
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <main class="login-page">
    <RouterLink class="back" to="/"><NIcon :component="ArrowLeft" />返回服务首页</RouterLink>
    <NCard class="login-card" :bordered="false">
      <div class="brand">
        <img src="../assets/logo.svg" alt="ServiceCompass" />
        <div>
          <p class="eyebrow">ServiceCompass</p>
          <h1>管理员登录</h1>
        </div>
      </div>
      <NForm @submit.prevent="submit">
        <NFormItem label="用户名">
          <NInput v-model:value="username" autocomplete="username" autofocus placeholder="请输入管理员用户名" />
        </NFormItem>
        <NFormItem label="密码">
          <NInput v-model:value="password" type="password" show-password-on="click" autocomplete="current-password" placeholder="请输入管理员密码" />
        </NFormItem>
        <NButton type="primary" block attr-type="submit" :loading="loading" :disabled="!username || !password">
          <template #icon><NIcon :component="Key" /></template>进入管理端
        </NButton>
      </NForm>
    </NCard>
  </main>
</template>

<style scoped>
.login-page { display: grid; min-height: 100vh; place-items: center; padding: 2rem; background: radial-gradient(circle at 70% 18%, rgb(52 211 153 / 10%), transparent 24rem); }
.login-card { width: min(25rem, 100%); padding: 1.35rem; box-shadow: 0 2rem 7rem rgb(0 0 0 / 42%); }
.brand { display: flex; align-items: center; gap: 0.9rem; margin-bottom: 1.5rem; }
.brand img { width: 3.2rem; }
.eyebrow { margin: 0 0 0.25rem; color: #5da9ff; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.16em; }
h1 { margin: 0; font-size: 1.7rem; }
.back { position: fixed; top: 2rem; left: 2rem; display: flex; align-items: center; gap: 0.4rem; color: #8190a5; text-decoration: none; }
</style>
