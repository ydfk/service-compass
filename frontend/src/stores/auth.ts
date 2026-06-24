import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api } from '../api/client'

export const useAuthStore = defineStore('auth', () => {
  const session = ref(localStorage.getItem('service-compass-session'))
  const username = ref('')
  const authenticated = computed(() => Boolean(session.value))

  async function login(loginUsername: string, password: string) {
    const response = await api<{ token: string; username: string }>('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username: loginUsername, password }),
    })
    session.value = response.token
    username.value = response.username
    localStorage.setItem('service-compass-session', response.token)
  }

  async function verify() {
    if (!session.value) return false
    try {
      const response = await api<{ username: string }>('/api/auth/me')
      username.value = response.username
      return true
    } catch {
      clear()
      return false
    }
  }

  async function logout() {
    try {
      await api('/api/auth/logout', { method: 'POST' })
    } finally {
      clear()
    }
  }

  function clear() {
    session.value = null
    username.value = ''
    localStorage.removeItem('service-compass-session')
  }

  return { session, username, authenticated, login, verify, logout, clear }
})
