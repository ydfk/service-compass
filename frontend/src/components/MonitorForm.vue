<script setup lang="ts">
import { NAlert, NForm, NFormItem, NInput, NInputNumber, NSelect, NSwitch } from 'naive-ui'
import { computed } from 'vue'
import type { MonitorInput, NotificationChannel, Service } from '../types'

const props = withDefaults(
  defineProps<{
    services: Service[]
    notificationChannels?: NotificationChannel[]
    showIdentity?: boolean
    showNotification?: boolean
    allowedTypes?: MonitorInput['monitor_type'][]
  }>(),
  {
    notificationChannels: () => [],
    showIdentity: true,
    showNotification: false,
    allowedTypes: () => ['http', 'http_keyword', 'dns', 'cert', 'docker'],
  },
)
const model = defineModel<MonitorInput>({ required: true })
const serviceOptions = computed(() => [
  { label: '不关联服务', value: '' },
  ...props.services.map((item) => ({ label: item.name, value: item.id })),
])
const typeOptions = computed(() =>
  [
    { label: 'HTTP / HTTPS', value: 'http' as const },
    { label: 'HTTP / HTTPS 关键字', value: 'http_keyword' as const },
    { label: 'DNS 解析', value: 'dns' as const },
    { label: 'HTTPS 证书', value: 'cert' as const },
    { label: 'Docker 容器状态', value: 'docker' as const },
  ].filter((item) => props.allowedTypes.includes(item.value)),
)
const channelOptions = computed(() =>
  props.notificationChannels.map((item) => ({ label: item.name, value: item.id })),
)
const ignoreTlsErrors = computed({
  get: () => !model.value.tls_verify,
  set: (value: boolean) => {
    model.value.tls_verify = !value
  },
})
</script>

<template>
  <NForm label-placement="top">
    <div v-if="showIdentity" class="form-grid">
      <NFormItem label="监控名称">
        <NInput v-model:value="model.name" placeholder="Plex 外网" />
      </NFormItem>
      <NFormItem label="所属服务">
        <NSelect
          :value="model.service_id || ''"
          :options="serviceOptions"
          @update:value="model.service_id = $event || null"
        />
      </NFormItem>
    </div>
    <div class="form-grid">
      <NFormItem label="监控类型">
        <NSelect
          v-model:value="model.monitor_type"
          :options="typeOptions"
        />
      </NFormItem>
      <NFormItem v-if="['http', 'http_keyword'].includes(model.monitor_type)" label="地址来源">
        <NSelect
          v-model:value="model.target_url_mode"
          :options="[
            { label: '自定义 URL', value: 'custom' },
            { label: '服务内网地址', value: 'local' },
            { label: '服务外网地址', value: 'public' },
          ]"
        />
      </NFormItem>
    </div>
    <NAlert v-if="model.monitor_type === 'docker'" type="info" :bordered="false">
      Docker 监控读取服务所关联容器的运行状态与 Health Check。
    </NAlert>
    <NFormItem
      v-if="['http', 'http_keyword'].includes(model.monitor_type) && model.target_url_mode === 'custom'"
      label="目标 URL"
    >
      <NInput v-model:value="model.target_url" placeholder="https://service.example.com/health" />
    </NFormItem>
    <NFormItem v-if="model.monitor_type === 'http_keyword'" label="响应关键字">
      <NInput
        v-model:value="model.keyword"
        placeholder="healthy"
      />
      <template #feedback>在纯 HTML 或 JSON 响应中搜索关键字，区分大小写</template>
    </NFormItem>
    <NFormItem v-if="['dns', 'cert'].includes(model.monitor_type)" label="域名">
      <NInput v-model:value="model.domain" placeholder="example.com" />
    </NFormItem>
    <div v-if="model.monitor_type === 'dns'" class="form-grid">
      <NFormItem label="记录类型">
        <NSelect
          v-model:value="model.record_type"
          :options="['A', 'AAAA', 'CNAME'].map((value) => ({ label: value, value }))"
        />
      </NFormItem>
      <NFormItem label="预期值（可选）">
        <NInput v-model:value="model.expected_value" placeholder="不填写则只检查能否解析" />
      </NFormItem>
    </div>
    <div v-if="model.monitor_type === 'cert'" class="form-grid">
      <NFormItem label="HTTPS 端口">
        <NInputNumber v-model:value="model.cert_port" :min="1" :max="65535" />
      </NFormItem>
      <NAlert type="info" :bordered="false">证书到期提醒提前天数在「设置」中统一配置。</NAlert>
    </div>
    <div class="form-grid three">
      <NFormItem label="检查间隔（秒）">
        <NInputNumber v-model:value="model.interval_sec" :min="5" />
      </NFormItem>
      <NFormItem label="超时（秒）">
        <NInputNumber v-model:value="model.timeout_sec" :min="1" />
      </NFormItem>
      <NFormItem label="重试次数">
        <NInputNumber v-model:value="model.retries" :min="0" :max="10" />
      </NFormItem>
      <NFormItem v-if="['http', 'http_keyword'].includes(model.monitor_type)" label="最小状态码">
        <NInputNumber v-model:value="model.expected_status_min" :min="100" :max="599" />
      </NFormItem>
      <NFormItem v-if="['http', 'http_keyword'].includes(model.monitor_type)" label="最大状态码">
        <NInputNumber v-model:value="model.expected_status_max" :min="100" :max="599" />
      </NFormItem>
      <NFormItem v-if="['http', 'http_keyword'].includes(model.monitor_type)" label="请求方法">
        <NSelect v-model:value="model.method" :options="['GET', 'HEAD', 'POST'].map((value) => ({ label: value, value }))" />
      </NFormItem>
    </div>
    <template v-if="['http', 'http_keyword'].includes(model.monitor_type) && model.method === 'POST'">
      <NFormItem label="请求体编码">
        <NSelect
          v-model:value="model.request_body_type"
          :options="[
            { label: 'JSON', value: 'json' },
            { label: 'x-www-form-urlencoded', value: 'form' },
          ]"
        />
      </NFormItem>
      <NFormItem label="请求体">
        <NInput
          v-model:value="model.request_body"
          type="textarea"
          placeholder='例如：{"status":"ok"} 或 a=1&b=2'
        />
      </NFormItem>
      <NFormItem label="请求头 JSON">
        <NInput
          v-model:value="model.request_headers"
          type="textarea"
          placeholder='例如：{"Authorization":"Bearer ..."}'
        />
      </NFormItem>
    </template>
    <div v-if="['http', 'http_keyword'].includes(model.monitor_type)" class="form-grid">
      <NFormItem label="验证">
        <NSelect
          v-model:value="model.auth_type"
          :options="[
            { label: '无', value: 'none' },
            { label: 'HTTP 基础身份验证', value: 'basic' },
          ]"
        />
      </NFormItem>
      <NFormItem v-if="model.auth_type === 'basic'" label="用户名">
        <NInput v-model:value="model.auth_username" />
      </NFormItem>
      <NFormItem v-if="model.auth_type === 'basic'" label="密码（留空则保留）">
        <NInput v-model:value="model.auth_password" type="password" show-password-on="click" />
      </NFormItem>
    </div>
    <div class="switches">
      <template v-if="['http', 'http_keyword'].includes(model.monitor_type)">
        <label><NSwitch v-model:value="ignoreTlsErrors" /> 忽略 HTTPS TLS/SSL 错误</label>
      </template>
      <label><NSwitch v-model:value="model.enabled" /> 启用监控</label>
    </div>
    <div v-if="showNotification" class="notify-box">
      <div class="switches">
        <label><NSwitch v-model:value="model.notify_enabled" /> 状态变化时通知</label>
      </div>
      <template v-if="model.notify_enabled">
        <NFormItem label="通知通道">
          <NSelect
            v-model:value="model.notification_channel_ids"
            :options="channelOptions"
            multiple
            filterable
            placeholder="选择一个或多个通知通道"
          />
        </NFormItem>
      </template>
    </div>
  </NForm>
</template>

<style scoped>
.form-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0 1rem;
}

.form-grid.three {
  grid-template-columns: repeat(3, 1fr);
}

.switches {
  display: flex;
  flex-wrap: wrap;
  gap: 1.2rem;
  margin: 0.5rem 0 1.5rem;
  color: #8594a8;
}

.switches label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.notify-box {
  padding-top: 0.6rem;
  border-top: 1px solid rgb(148 163 184 / 12%);
}

@media (max-width: 680px) {
  .form-grid,
  .form-grid.three {
    grid-template-columns: 1fr;
  }
}
</style>
