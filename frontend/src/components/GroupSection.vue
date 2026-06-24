<script setup lang="ts">
import { Plus } from '@vicons/tabler'
import { NButton, NIcon } from 'naive-ui'
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
  clone: [service: Service]
  add: [group: DashboardGroup]
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
        @clone="emit('clone', $event)"
        @move="(item, direction) => emit('move', group, item, direction)"
      />
      <button v-if="editable && !sorting" class="add-service" type="button" @click="emit('add', group)">
        <NIcon :component="Plus" />
        <span>添加服务</span>
      </button>
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
.add-service { display: flex; min-height: 3.8rem; align-items: center; justify-content: center; gap: 0.45rem; border: 1px dashed rgb(96 165 250 / 30%); border-radius: 0.8rem; background: transparent; color: #70829a; cursor: pointer; transition: 160ms ease; }
.detail .add-service { min-height: 11.5rem; }
.add-service:hover { border-color: rgb(96 165 250 / 65%); background: rgb(30 64 175 / 8%); color: #93c5fd; }
@media (max-width: 520px) { h2 small { display: block; margin: 0.25rem 0 0; } }
</style>
