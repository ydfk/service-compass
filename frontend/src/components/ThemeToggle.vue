<script setup lang="ts">
import { Moon, Sun } from '@vicons/tabler'
import { NButton, NIcon } from 'naive-ui'
import { useThemeStore } from '../stores/theme'

const theme = useThemeStore()
</script>

<template>
  <NButton size="small" @click="theme.toggle">
    <template #icon><NIcon :component="theme.isDark ? Sun : Moon" /></template>
    {{ theme.isDark ? '亮色' : '暗色' }}
  </NButton>
</template>
