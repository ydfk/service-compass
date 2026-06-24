<script setup lang="ts">
import { BuildingCommunity, World } from '@vicons/tabler'
import { NButton, NButtonGroup, NIcon } from 'naive-ui'
import type { UrlMode } from '../types'

defineProps<{ mode: UrlMode }>()
const emit = defineEmits<{ change: [mode: UrlMode] }>()
</script>

<template>
  <NButtonGroup size="small">
    <NButton :type="mode === 'local' ? 'primary' : 'default'" @click="emit('change', 'local')">
      <template #icon><NIcon :component="BuildingCommunity" /></template>内网
    </NButton>
    <NButton :type="mode === 'public' ? 'primary' : 'default'" @click="emit('change', 'public')">
      <template #icon><NIcon :component="World" /></template>外网
    </NButton>
  </NButtonGroup>
</template>

