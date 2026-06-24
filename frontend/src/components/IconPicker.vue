<script setup lang="ts">
import { ExternalLink, Photo, Search, Upload } from '@vicons/tabler'
import { NButton, NIcon, NInput, NSpace, useMessage } from 'naive-ui'
import { computed, ref } from 'vue'
import { iconsApi } from '../api/icons'

const props = defineProps<{
  name: string
  iconType: string
  iconValue?: string | null
  serviceUrl?: string | null
}>()
const emit = defineEmits<{
  'update:iconType': [value: string]
  'update:iconValue': [value: string]
}>()
const loading = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)
const message = useMessage()

const preview = computed(() => {
  if (!props.iconValue) return null
  return props.iconType === 'selfhst'
    ? `https://cdn.jsdelivr.net/gh/selfhst/icons/svg/${props.iconValue}.svg`
    : props.iconValue
})

async function suggest() {
  if (!props.name.trim()) return message.warning('请先填写服务名称')
  loading.value = true
  try {
    const suggestion = await iconsApi.suggest(props.name)
    const result = await iconsApi.test(suggestion.reference)
    emit('update:iconType', 'selfhst')
    emit('update:iconValue', suggestion.reference)
    message.success(`已匹配 ${suggestion.reference} · ${result.url.split('/').at(-1)}`)
  } finally {
    loading.value = false
  }
}

async function upload(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  loading.value = true
  try {
    const result = await iconsApi.upload(file)
    emit('update:iconType', 'upload')
    emit('update:iconValue', result.url)
    message.success('图标已上传')
  } finally {
    loading.value = false
    input.value = ''
  }
}

async function favicon() {
  if (!props.serviceUrl) return message.warning('请先填写服务地址')
  loading.value = true
  try {
    const result = await iconsApi.favicon(props.serviceUrl)
    emit('update:iconType', 'favicon')
    emit('update:iconValue', result.url)
    message.success('已发现 favicon')
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="icon-picker">
    <div class="preview">
      <img v-if="preview" :src="preview" alt="图标预览" />
      <NIcon v-else :component="Photo" />
    </div>
    <div class="controls">
      <NInput
        :value="iconValue || ''"
        placeholder="selfh.st reference 或图标 URL"
        @update:value="emit('update:iconValue', $event)"
      />
      <NSpace>
        <NButton size="small" :loading="loading" @click="suggest">
          <template #icon><NIcon :component="Photo" /></template>匹配 selfh.st
        </NButton>
        <a class="icon-library-link" href="https://selfh.st/icons/" target="_blank" rel="noopener noreferrer">
          <NIcon :component="ExternalLink" />浏览图标库
        </a>
        <NButton size="small" :loading="loading" @click="favicon">
          <template #icon><NIcon :component="Search" /></template>获取 favicon
        </NButton>
        <NButton size="small" :loading="loading" @click="fileInput?.click()">
          <template #icon><NIcon :component="Upload" /></template>上传图标
        </NButton>
        <input ref="fileInput" class="file-input" type="file" accept="image/png,image/jpeg,image/webp,image/x-icon" @change="upload" />
      </NSpace>
    </div>
  </div>
</template>

<style scoped>
.icon-picker {
  display: grid;
  grid-template-columns: 4rem 1fr;
  gap: 0.9rem;
  width: 100%;
}

.preview {
  display: grid;
  width: 4rem;
  height: 4rem;
  place-items: center;
  overflow: hidden;
  border: 1px solid rgb(148 163 184 / 18%);
  border-radius: 0.8rem;
  background: #080d17;
  font-size: 1.5rem;
}

.preview img {
  width: 2.8rem;
  height: 2.8rem;
  object-fit: contain;
}

.controls {
  display: grid;
  gap: 0.55rem;
}
.file-input { display: none; }
.icon-library-link { display: inline-flex; align-items: center; gap: 0.3rem; color: #75b7ff; font-size: 0.78rem; text-decoration: none; }
.icon-library-link:hover { text-decoration: underline; }
</style>
