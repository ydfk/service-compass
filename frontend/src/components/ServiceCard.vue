<script setup lang="ts">
import { ArrowLeft, ArrowRight, Copy, Edit, Photo } from '@vicons/tabler'
import { NButton, NIcon } from 'naive-ui'
import { computed, ref, watch } from 'vue'
import type { MonitorTrack, Service, StatusPoint, UrlMode } from '../types'
import StatusBadge from './StatusBadge.vue'
import StatusStrip from './StatusStrip.vue'

const props = defineProps<{
  service: Service
  mode: UrlMode
  cardMode: 'compact' | 'detail'
  editable?: boolean
  sorting?: boolean
  index: number
  total: number
}>()
const emit = defineEmits<{
  edit: [service: Service]
  clone: [service: Service]
  move: [service: Service, direction: -1 | 1]
}>()
const iconFailed = ref(false)
const activeUrl = computed(() => {
  const preferred = props.mode === 'local' ? props.service.local_url : props.service.public_url
  return preferred || props.service.public_url || props.service.local_url
})
const iconUrl = computed(() => props.service.icon_url || props.service.icon_value || '')

watch(iconUrl, () => {
  iconFailed.value = false
})

function open() {
  if (activeUrl.value && !props.sorting)
    window.open(activeUrl.value, '_blank', 'noopener,noreferrer')
}

function trackLabel(track: MonitorTrack) {
  if (track.monitor_type === 'docker') return 'Docker'
  if (track.monitor_type === 'http_keyword') return 'HTTP 关键字'
  return track.monitor_type.toUpperCase()
}

function trackPoints(track: MonitorTrack): StatusPoint[] {
  if (track.segment_details?.length) return track.segment_details
  return track.segments.map((status) => ({ status }))
}
</script>

<template>
  <article class="service-card" :class="[cardMode, { 'no-url': !activeUrl, sorting }]" @click="open">
    <div class="service-icon">
      <img v-if="iconUrl && !iconFailed" :src="iconUrl" :alt="service.name" @error="iconFailed = true" />
      <NIcon v-else :component="Photo" />
    </div>
    <div class="identity">
      <h3>{{ service.name }}</h3>
      <p v-if="cardMode === 'detail'">{{ service.description || service.docker_image || (activeUrl ? '服务状态' : '未配置访问地址') }}</p>
    </div>
    <StatusBadge :status="service.status" />

    <div v-if="cardMode === 'detail'" class="monitor-tracks">
      <div v-if="!service.monitor_tracks?.length" class="no-monitor">未配置监控</div>
      <div v-for="track in service.monitor_tracks" :key="track.id" class="track">
        <div class="track-meta">
          <span><i :class="track.status" />{{ trackLabel(track) }}</span>
          <span>{{ track.uptime_percent == null ? '等待数据' : `${track.uptime_percent.toFixed(2)}%` }}<template v-if="track.last_latency_ms != null"> · {{ track.last_latency_ms }}ms</template></span>
        </div>
        <StatusStrip :points="trackPoints(track)" title="过去 24 小时最近 30 次检查" />
      </div>
    </div>
    <span v-if="!activeUrl" class="no-url-badge">{{ cardMode === 'detail' ? '仅展示' : '无链接' }}</span>

    <div v-if="editable" class="card-tools" @click.stop>
      <template v-if="sorting">
        <NButton quaternary circle size="tiny" :disabled="index === 0" title="向前移动" @click="emit('move', service, -1)"><NIcon :component="ArrowLeft" /></NButton>
        <NButton quaternary circle size="tiny" :disabled="index === total - 1" title="向后移动" @click="emit('move', service, 1)"><NIcon :component="ArrowRight" /></NButton>
      </template>
      <template v-else>
        <NButton quaternary circle size="tiny" title="克隆服务" @click="emit('clone', service)"><NIcon :component="Copy" /></NButton>
        <NButton quaternary circle size="tiny" title="编辑服务" @click="emit('edit', service)"><NIcon :component="Edit" /></NButton>
      </template>
    </div>
  </article>
</template>

<style scoped>
.service-card { position: relative; display: grid; grid-template-columns: 2.4rem minmax(0, 1fr) auto; align-items: center; gap: 0.75rem; padding: 0.7rem 0.8rem; border: 1px solid var(--sc-border); border-radius: 0.8rem; background: var(--sc-card); cursor: pointer; transition: border-color 160ms ease, background 160ms ease, transform 160ms ease; }
.service-card:hover { border-color: rgb(96 165 250 / 42%); background: var(--sc-card-hover); transform: translateY(-1px); }
.service-card.no-url { border-style: dashed; cursor: default; }
.service-card.no-url::after { position: absolute; inset: 0; border-radius: inherit; background: linear-gradient(135deg, transparent, rgb(148 163 184 / 5%)); content: ""; pointer-events: none; }
.service-card.sorting { cursor: move; border-style: dashed; }
.service-card.detail { grid-template-columns: 3rem minmax(0, 1fr) auto; align-content: start; min-height: 11.5rem; padding: 1rem; }
.service-icon { display: grid; width: 2.4rem; height: 2.4rem; place-items: center; border-radius: 0.6rem; background: var(--sc-icon-bg); font-size: 1.2rem; }
.detail .service-icon { width: 3rem; height: 3rem; }
.service-icon img { width: 72%; height: 72%; object-fit: contain; }
.identity { min-width: 0; }
h3 { margin: 0; overflow: hidden; font-size: 0.95rem; text-overflow: ellipsis; white-space: nowrap; }
.detail h3 { font-size: 1.05rem; }
.identity p { margin: 0.28rem 0 0; overflow: hidden; color: var(--sc-muted); font-size: 0.72rem; text-overflow: ellipsis; white-space: nowrap; }
.monitor-tracks { display: grid; grid-column: 1 / -1; gap: 0.75rem; margin-top: 0.45rem; }
.track { display: grid; gap: 0.35rem; }
.track-meta { display: flex; justify-content: space-between; gap: 1rem; color: var(--sc-muted); font-family: "IBM Plex Mono", monospace; font-size: 0.65rem; }
.track-meta span { display: flex; align-items: center; gap: 0.35rem; }
.track-meta i { width: 0.4rem; height: 0.4rem; border-radius: 50%; background: #64748b; }
.track-meta i.up { background: #34d399; }.track-meta i.down { background: #fb7185; }.track-meta i.warning { background: #fbbf24; }
.no-monitor { padding: 0.65rem; border: 1px dashed rgb(148 163 184 / 15%); border-radius: 0.5rem; color: var(--sc-muted); font-size: 0.72rem; text-align: center; }
.no-url-badge { position: absolute; right: 0.75rem; bottom: 0.55rem; color: var(--sc-subtle); font-size: 0.66rem; }
.card-tools { position: absolute; top: -0.45rem; right: -0.45rem; display: flex; border: 1px solid rgb(148 163 184 / 18%); border-radius: 999px; background: var(--sc-surface); }
@media (hover: hover) { .card-tools { opacity: 0; transition: opacity 160ms ease; } .service-card:hover .card-tools, .card-tools:focus-within { opacity: 1; } }
</style>
