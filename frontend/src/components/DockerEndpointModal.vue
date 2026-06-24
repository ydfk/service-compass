<script setup lang="ts">
import { NButton, NForm, NFormItem, NInput, NModal, NSelect, NSwitch, useMessage } from 'naive-ui'
import { ref, watch } from 'vue'
import { dockerApi } from '../api/docker'
import type { DockerEndpoint, DockerEndpointInput } from '../types'

const props = defineProps<{ endpoint?: DockerEndpoint | null }>()
const emit = defineEmits<{ saved: [endpoint: DockerEndpoint] }>()
const show = defineModel<boolean>('show', { required: true })
const form = ref<DockerEndpointInput>(emptyEndpoint())
const saving = ref(false)
const message = useMessage()

watch(show, (value) => {
  if (!value) return
  form.value = props.endpoint ? endpointToInput(props.endpoint) : emptyEndpoint()
})

function emptyEndpoint(): DockerEndpointInput {
  return {
    name: '本机 Docker',
    endpoint_type: 'local_socket',
    endpoint_url: 'unix:///var/run/docker.sock',
    tls_enabled: false,
    tls_ca: '',
    tls_cert: '',
    tls_key: '',
    lan_host: '',
    public_host_hint: '',
    enabled: true,
  }
}

function endpointToInput(endpoint: DockerEndpoint): DockerEndpointInput {
  return {
    name: endpoint.name,
    endpoint_type: endpoint.endpoint_type,
    endpoint_url: endpoint.endpoint_url,
    tls_enabled: endpoint.tls_enabled,
    tls_ca: '',
    tls_cert: '',
    tls_key: '',
    lan_host: endpoint.lan_host,
    public_host_hint: endpoint.public_host_hint,
    enabled: endpoint.enabled,
  }
}

async function save() {
  if (!form.value.name.trim() || !form.value.endpoint_url.trim()) {
    return message.warning('请填写名称与 Endpoint 地址')
  }
  saving.value = true
  try {
    const endpoint = props.endpoint
      ? await dockerApi.updateEndpoint(props.endpoint.id, form.value)
      : await dockerApi.createEndpoint(form.value)
    show.value = false
    emit('saved', endpoint)
    message.success('Docker Endpoint 已保存')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <NModal v-model:show="show" preset="card" :title="endpoint ? '编辑 Docker Endpoint' : '添加 Docker Endpoint'" class="endpoint-modal">
    <NForm label-placement="top">
      <div class="two-columns">
        <NFormItem label="名称"><NInput v-model:value="form.name" placeholder="例如：本机 Docker" /></NFormItem>
        <NFormItem label="类型"><NSelect v-model:value="form.endpoint_type" :options="[{ label: '本机 Unix Socket', value: 'local_socket' }, { label: '远程 TCP API', value: 'remote_tcp' }]" /></NFormItem>
      </div>
      <NFormItem label="Endpoint 地址"><NInput v-model:value="form.endpoint_url" :placeholder="form.endpoint_type === 'local_socket' ? 'unix:///var/run/docker.sock' : 'tcp://10.0.0.251:2376'" /></NFormItem>
      <div class="two-columns">
        <NFormItem label="局域网主机（可选）"><NInput v-model:value="form.lan_host" placeholder="例如：10.0.0.251" /><small class="field-help">与容器发布端口组合为候选内网地址，不是 Docker API 地址。</small></NFormItem>
        <NFormItem label="外网主机（可选）"><NInput v-model:value="form.public_host_hint" placeholder="例如：service.example.com" /><small class="field-help">仅用于生成候选外网地址，不会自动配置域名或反向代理。</small></NFormItem>
      </div>
      <NFormItem v-if="form.endpoint_type === 'remote_tcp'" label="TLS"><NSwitch v-model:value="form.tls_enabled" /></NFormItem>
      <template v-if="form.endpoint_type === 'remote_tcp' && form.tls_enabled">
        <NFormItem label="TLS CA（PEM，留空保留）"><NInput v-model:value="form.tls_ca" type="textarea" placeholder="粘贴 CA 证书" /></NFormItem>
        <NFormItem label="TLS Cert（PEM，留空保留）"><NInput v-model:value="form.tls_cert" type="textarea" placeholder="粘贴客户端证书" /></NFormItem>
        <NFormItem label="TLS Key（PEM，留空保留）"><NInput v-model:value="form.tls_key" type="textarea" placeholder="粘贴客户端私钥" /></NFormItem>
      </template>
      <NButton type="primary" block :loading="saving" @click="save">保存 Endpoint</NButton>
    </NForm>
  </NModal>
</template>

<style scoped>
.two-columns { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }
.field-help { display: block; margin-top: 0.35rem; color: #6f8098; line-height: 1.45; }
@media (max-width: 760px) { .two-columns { grid-template-columns: 1fr; } }
</style>
