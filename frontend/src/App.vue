<script setup lang="ts">
import {
  NConfigProvider,
  NDialogProvider,
  NGlobalStyle,
  NMessageProvider,
  dateZhCN,
  darkTheme,
  zhCN,
} from 'naive-ui'
import { storeToRefs } from 'pinia'
import { computed, watchEffect } from 'vue'
import { RouterView } from 'vue-router'
import { darkThemeOverrides, lightThemeOverrides } from './styles/theme'
import { useThemeStore } from './stores/theme'

const theme = useThemeStore()
const { isDark, mode } = storeToRefs(theme)
const naiveTheme = computed(() => (isDark.value ? darkTheme : null))
const overrides = computed(() => (isDark.value ? darkThemeOverrides : lightThemeOverrides))

watchEffect(() => {
  document.documentElement.classList.toggle('theme-light', mode.value === 'light')
  document.documentElement.classList.toggle('theme-dark', mode.value !== 'light')
})
</script>

<template>
  <NConfigProvider :theme="naiveTheme" :theme-overrides="overrides" :locale="zhCN" :date-locale="dateZhCN">
    <NGlobalStyle />
    <NMessageProvider>
      <NDialogProvider>
        <RouterView />
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>
