<script setup lang="ts">
import DataMaintenancePanel from '../components/DataMaintenancePanel.vue'
</script>

<template>
  <header class="page-header">
    <p>DATA SAFETY</p>
    <h1>备份</h1>
    <span>导出、导入、测试备份目标，并查看备份任务日志。</span>
  </header>
  <DataMaintenancePanel />
</template>

<style scoped>
.page-header p {
  margin: 0;
  color: #5da9ff;
  font-family: "IBM Plex Mono", monospace;
  font-size: 0.68rem;
  letter-spacing: 0.2em;
}

.page-header h1 {
  margin: 0.35rem 0;
  font-size: 2.1rem;
}

.page-header span {
  color: #75859b;
}
</style>
