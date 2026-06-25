import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export type ThemeMode = 'dark' | 'light'

const STORAGE_KEY = 'service-compass-theme'

export const useThemeStore = defineStore('theme', () => {
  const mode = ref<ThemeMode>((localStorage.getItem(STORAGE_KEY) as ThemeMode) || 'dark')
  const isDark = computed(() => mode.value !== 'light')

  function setMode(value: ThemeMode) {
    mode.value = value
    localStorage.setItem(STORAGE_KEY, value)
  }

  function toggle() {
    setMode(isDark.value ? 'light' : 'dark')
  }

  return { mode, isDark, setMode, toggle }
})
