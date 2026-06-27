<script setup lang="ts">
import { ExternalLink, Photo, Search, Square, Upload } from '@vicons/tabler'
import { NButton, NIcon, NInput, NSelect, NSpace, useMessage } from 'naive-ui'
import { computed, ref, watch } from 'vue'
import { iconsApi } from '../api/icons'

const props = defineProps<{
  name: string
  iconType: string
  iconValue?: string | null
  serviceUrl?: string | null
  authType?: 'none' | 'basic'
  authUsername?: string | null
  authPassword?: string | null
}>()
const emit = defineEmits<{
  'update:iconType': [value: string]
  'update:iconValue': [value: string]
}>()
const loading = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)
const previewFailed = ref(false)
const faviconOptions = ref<string[]>([])
const selfhstController = ref<AbortController | null>(null)
const iconKeyword = ref('')
const message = useMessage()

const preview = computed(() => {
  if (!props.iconValue) return null
  return props.iconType === 'selfhst'
    ? `https://cdn.jsdelivr.net/gh/selfhst/icons/svg/${props.iconValue}.svg`
    : props.iconValue
})
const faviconSelectOptions = computed(() =>
  faviconOptions.value.map((url) => ({ label: faviconLabel(url), value: url })),
)

watch(preview, () => {
  previewFailed.value = false
})

async function suggest() {
  const keyword = (iconKeyword.value || props.name).trim()
  if (!keyword) return message.warning('请先填写图标关键词')
  selfhstController.value?.abort()
  const controller = new AbortController()
  selfhstController.value = controller
  loading.value = true
  try {
    const suggestion = await iconsApi.suggest(keyword)
    try {
      const result = await iconsApi.test(suggestion.reference, controller.signal)
      emit('update:iconType', 'upload')
      emit('update:iconValue', result.url)
      message.success(`已下载 ${suggestion.reference} 到本地图标库`)
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') {
        message.info('已停止匹配 selfh.st')
        return
      }
      message.warning(`未匹配到“${suggestion.reference}”，请浏览图标库或手动输入正确的 reference`)
    }
  } finally {
    if (selfhstController.value === controller) {
      loading.value = false
      selfhstController.value = null
    }
  }
}

function stopSuggest() {
  selfhstController.value?.abort()
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
    const result = await iconsApi.favicon({
      url: props.serviceUrl,
      auth_type: props.authType,
      auth_username: props.authUsername,
      auth_password: props.authPassword,
    })
    faviconOptions.value = result.urls
    if (result.urls.length === 0) {
      emit('update:iconType', 'auto')
      emit('update:iconValue', '')
      message.warning('未发现 favicon，已清空当前图标')
      return
    }
    selectFavicon(result.urls[0])
    message.success(
      result.urls.length > 1 ? `发现 ${result.urls.length} 个 favicon，请选择` : '已发现 favicon',
    )
  } finally {
    loading.value = false
  }
}

function selectFavicon(url?: string | null) {
  if (!url) return
  emit('update:iconType', 'favicon')
  emit('update:iconValue', url)
}

function faviconLabel(value: string) {
  try {
    const url = new URL(value)
    return `${url.hostname} · ${url.pathname.split('/').at(-1) || 'favicon'}`
  } catch {
    return value
  }
}
</script>

<template>
  <div class="icon-picker">
    <div class="preview">
      <img v-if="preview && !previewFailed" :src="preview" alt="图标预览" @error="previewFailed = true" />
      <NIcon v-else :component="Photo" />
    </div>
    <div class="controls">
      <NInput
        :value="iconValue || ''"
        placeholder="selfh.st reference 或图标 URL"
        @update:value="emit('update:iconValue', $event)"
      />
      <NInput v-model:value="iconKeyword" placeholder="图标关键词，例如 MoviePilot、Syncthing" />
      <NSelect
        v-if="faviconOptions.length > 1"
        :value="iconType === 'favicon' ? iconValue : null"
        :options="faviconSelectOptions"
        placeholder="选择一个 favicon"
        @update:value="selectFavicon"
      />
      <NSpace>
        <NButton size="small" :loading="loading" @click="suggest">
          <template #icon><NIcon :component="Photo" /></template>匹配 selfh.st
        </NButton>
        <NButton v-if="selfhstController" size="small" type="warning" secondary @click="stopSuggest">
          <template #icon><NIcon :component="Square" /></template>停止
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
        <input
          ref="fileInput"
          class="file-input"
          type="file"
          accept="image/png,image/jpeg,image/webp,image/svg+xml,.svg,image/x-icon"
          @change="upload"
        />
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
