<script setup lang="ts">
import { ArrowLeft, Key } from '@vicons/tabler'
import { NButton, NCard, NForm, NFormItem, NIcon, NInput, useMessage } from 'naive-ui'
import { ref } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const username = ref('admin')
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
      <img src="../assets/logo.svg" alt="ServiceCompass" />
      <p class="eyebrow">ADMIN CONSOLE</p>
      <h1>校准管理航向</h1>
      <p class="intro">使用管理员账号进入控制台。首次登录账号和密码均为 admin。</p>
      <NForm @submit.prevent="submit">
        <NFormItem label="用户名">
          <NInput v-model:value="username" autocomplete="username" autofocus />
        </NFormItem>
        <NFormItem label="密码">
          <NInput v-model:value="password" type="password" show-password-on="click" autocomplete="current-password" />
        </NFormItem>
        <NButton type="primary" block attr-type="submit" :loading="loading" :disabled="!username || !password">
          <template #icon><NIcon :component="Key" /></template>进入管理端
        </NButton>
      </NForm>
    </NCard>
  </main>
</template>

<style scoped>
.login-page { display: grid; min-height: 100vh; place-items: center; padding: 2rem; background: radial-gradient(circle at 72% 24%, rgb(52 211 153 / 10%), transparent 24rem); }
.login-card { width: min(28rem, 100%); padding: 1.5rem; box-shadow: 0 2rem 7rem rgb(0 0 0 / 45%); }
.login-card img { width: 4.2rem; }
.eyebrow { margin: 1.6rem 0 0.5rem; color: #5da9ff; font-family: "IBM Plex Mono", monospace; font-size: 0.68rem; letter-spacing: 0.2em; }
h1 { margin: 0; font-size: 2rem; }
.intro { margin: 0.8rem 0 1.8rem; color: #8190a5; line-height: 1.6; }
.back { position: fixed; top: 2rem; left: 2rem; display: flex; align-items: center; gap: 0.4rem; color: #8190a5; text-decoration: none; }
</style>
