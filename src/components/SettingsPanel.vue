<template>
  <div class="settings-panel">
    <div class="settings-header">
      <h3 class="settings-title">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
          <path d="M12 15a3 3 0 100-6 3 3 0 000 6z" stroke="currentColor" stroke-width="1.5"/>
          <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z" stroke="currentColor" stroke-width="1.5"/>
        </svg>
        账户设置
      </h3>
      <div class="conn-status" :class="connected ? 'online' : 'offline'">
        <span class="status-dot"></span>
        {{ connected ? '已连接' : '未连接' }}
      </div>
    </div>

    <div class="settings-body">
      <!-- 快捷预设 -->
      <div class="settings-section">
        <h4 class="section-title">快捷配置</h4>
        <div class="preset-grid">
          <button
            v-for="preset in presets"
            :key="preset.name"
            class="preset-btn"
            @click="applyPreset(preset)"
          >
            <span class="preset-icon">{{ preset.icon }}</span>
            <span class="preset-name">{{ preset.name }}</span>
          </button>
        </div>
      </div>

      <!-- IMAP 配置 -->
      <div class="settings-section">
        <h4 class="section-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
            <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            <path d="M7 10l5 5 5-5M12 15V3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          收件服务器 (IMAP)
        </h4>
        <div class="field-group">
          <div class="field-row">
            <div class="field">
              <label>服务器地址</label>
              <input v-model="localConfig.imap.host" placeholder="imap.example.com" />
            </div>
            <div class="field small">
              <label>端口</label>
              <input v-model.number="localConfig.imap.port" type="number" placeholder="993" />
            </div>
          </div>
          <div class="field-row">
            <div class="field">
              <label>用户名 / 邮箱</label>
              <input v-model="localConfig.imap.user" placeholder="your@email.com" />
            </div>
          </div>
          <div class="field-row">
            <div class="field">
              <label>密码 / 授权码</label>
              <input v-model="localConfig.imap.pass" type="password" placeholder="密码或应用授权码" />
            </div>
          </div>
          <label class="checkbox-row">
            <input type="checkbox" v-model="localConfig.imap.secure" />
            <span>使用 SSL/TLS 加密</span>
          </label>
        </div>
      </div>

      <!-- SMTP 配置 -->
      <div class="settings-section">
        <h4 class="section-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
            <path d="M22 2L11 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            <path d="M22 2L15 22l-4-9-9-4L22 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
          </svg>
          发件服务器 (SMTP)
        </h4>
        <div class="field-group">
          <div class="field-row">
            <div class="field">
              <label>服务器地址</label>
              <input v-model="localConfig.smtp.host" placeholder="smtp.example.com" />
            </div>
            <div class="field small">
              <label>端口</label>
              <input v-model.number="localConfig.smtp.port" type="number" placeholder="465" />
            </div>
          </div>
          <div class="field-row">
            <div class="field">
              <label>用户名 / 邮箱</label>
              <input v-model="localConfig.smtp.user" placeholder="your@email.com" />
            </div>
          </div>
          <div class="field-row">
            <div class="field">
              <label>密码 / 授权码</label>
              <input v-model="localConfig.smtp.pass" type="password" placeholder="密码或应用授权码" />
            </div>
          </div>
          <label class="checkbox-row">
            <input type="checkbox" v-model="localConfig.smtp.secure" />
            <span>使用 SSL/TLS 加密</span>
          </label>
        </div>
      </div>

      <!-- 操作按钮 -->
      <div class="settings-actions">
        <button class="btn-primary" @click="handleSave">保存配置</button>
        <button v-if="!connected" class="btn-glass" @click="handleConnect">连接邮箱</button>
        <button v-else class="btn-glass danger-text" @click="$emit('disconnect')">断开连接</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { reactive } from 'vue'

const props = defineProps({
  accountConfig: Object,
  connected: Boolean,
})

const emit = defineEmits(['save', 'connect', 'disconnect'])

const localConfig = reactive({
  imap: { ...props.accountConfig.imap },
  smtp: { ...props.accountConfig.smtp },
})

const presets = [
  {
    name: 'QQ邮箱',
    icon: '📧',
    imap: { host: 'imap.qq.com', port: 993, secure: true },
    smtp: { host: 'smtp.qq.com', port: 465, secure: true },
  },
  {
    name: '163邮箱',
    icon: '📨',
    imap: { host: 'imap.163.com', port: 993, secure: true },
    smtp: { host: 'smtp.163.com', port: 465, secure: true },
  },
  {
    name: 'Gmail',
    icon: '✉️',
    imap: { host: 'imap.gmail.com', port: 993, secure: true },
    smtp: { host: 'smtp.gmail.com', port: 465, secure: true },
  },
  {
    name: 'Outlook',
    icon: '📬',
    imap: { host: 'outlook.office365.com', port: 993, secure: true },
    smtp: { host: 'smtp.office365.com', port: 587, secure: false },
  },
  {
    name: '126邮箱',
    icon: '📩',
    imap: { host: 'imap.126.com', port: 993, secure: true },
    smtp: { host: 'smtp.126.com', port: 465, secure: true },
  },
  {
    name: '新浪邮箱',
    icon: '📮',
    imap: { host: 'imap.sina.com', port: 993, secure: true },
    smtp: { host: 'smtp.sina.com', port: 465, secure: true },
  },
]

function applyPreset(preset) {
  localConfig.imap.host = preset.imap.host
  localConfig.imap.port = preset.imap.port
  localConfig.imap.secure = preset.imap.secure
  localConfig.smtp.host = preset.smtp.host
  localConfig.smtp.port = preset.smtp.port
  localConfig.smtp.secure = preset.smtp.secure
}

function handleSave() {
  emit('save', {
    imap: { ...localConfig.imap },
    smtp: { ...localConfig.smtp },
  })
}

function handleConnect() {
  handleSave()
  emit('connect')
}
</script>

<style scoped>
.settings-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 24px;
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
}

.settings-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  display: flex;
  align-items: center;
  gap: 10px;
}

.conn-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  padding: 4px 12px;
  border-radius: var(--radius-lg);
}

.conn-status.online {
  background: rgba(85, 239, 196, 0.12);
  color: var(--success);
}

.conn-status.offline {
  background: var(--glass-bg);
  color: var(--text-tertiary);
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}

.online .status-dot {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}

.offline .status-dot {
  background: var(--text-tertiary);
}

.settings-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}

.settings-section {
  margin-bottom: 28px;
}

.section-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 8px;
}

/* 预设按钮 */
.preset-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

.preset-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 14px 8px;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-md);
  color: var(--text-secondary);
  font-size: 12px;
  transition: all var(--transition-fast);
}

.preset-btn:hover {
  background: rgba(108, 92, 231, 0.10);
  border-color: rgba(108, 92, 231, 0.25);
  color: var(--text-primary);
}

.preset-icon {
  font-size: 22px;
}

.preset-name {
  font-weight: 500;
}

/* 表单 */
.field-group {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.field-row {
  display: flex;
  gap: 10px;
}

.field {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field.small {
  max-width: 100px;
}

.field label {
  font-size: 11px;
  color: var(--text-tertiary);
  font-weight: 500;
}

.field input {
  padding: 9px 12px;
  background: var(--input-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 13px;
  transition: border-color var(--transition-fast);
}

.field input:focus {
  border-color: var(--primary);
}

.checkbox-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 4px 0;
}

.checkbox-row input[type="checkbox"] {
  accent-color: var(--primary);
}

/* 操作按钮 */
.settings-actions {
  display: flex;
  gap: 10px;
  padding-top: 8px;
}

.settings-actions .btn-primary {
  padding: 10px 24px;
  border-radius: var(--radius-lg);
}

.settings-actions .btn-glass {
  padding: 10px 24px;
  border-radius: var(--radius-lg);
}

.danger-text {
  color: var(--danger) !important;
}

.danger-text:hover {
  background: rgba(255, 107, 107, 0.12) !important;
}
</style>
