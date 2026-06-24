<script setup lang="ts">
import type { DashboardGroup, Service, UrlMode } from '../types'
import ServiceCard from './ServiceCard.vue'

defineProps<{
  group: DashboardGroup
  mode: UrlMode
  cardMode: 'compact' | 'detail'
  editable?: boolean
  sorting?: boolean
}>()
const emit = defineEmits<{
  edit: [service: Service]
  move: [group: DashboardGroup, service: Service, direction: -1 | 1]
}>()
</script>

<template>
  <section class="group-section">
    <header>
      <h2>{{ group.name }} <small>{{ group.services.filter((item) => item.status === 'up').length }} 在线 / {{ group.services.length }} 服务</small></h2>
    </header>
    <div class="service-grid" :class="cardMode">
      <ServiceCard
        v-for="(service, index) in group.services"
        :key="service.id"
        :service="service"
        :mode="mode"
        :card-mode="cardMode"
        :editable="editable"
        :sorting="sorting"
        :index="index"
        :total="group.services.length"
        @edit="emit('edit', $event)"
        @move="(item, direction) => emit('move', group, item, direction)"
      />
    </div>
  </section>
</template>

<style scoped>
.group-section { margin-top: 1.8rem; }
header { margin-bottom: 0.8rem; }
h2 { margin: 0; font-size: 1.2rem; letter-spacing: -0.02em; }
h2 small { margin-left: 0.55rem; color: #65758c; font-size: 0.72rem; font-weight: 400; letter-spacing: 0; }
.service-grid { display: grid; gap: 0.7rem; }
.service-grid.compact { grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr)); }
.service-grid.detail { grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr)); }
@media (max-width: 520px) { h2 small { display: block; margin: 0.25rem 0 0; } }
</style>
