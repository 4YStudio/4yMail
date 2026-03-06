<template>
  <div class="settings-panel">
    <div class="settings-header">
      <div class="settings-tabs">
        <button 
          class="tab-btn" 
          :class="{ active: currentTab === 'accounts' }"
          @click="currentTab = 'accounts'; editingAccount = null"
        >
          邮件账户
        </button>
        <button 
          class="tab-btn" 
          :class="{ active: currentTab === 'general' }"
          @click="currentTab = 'general'"
        >
          通用设置
        </button>
      </div>
      <div class="header-right-actions">
        <button v-if="currentTab === 'accounts' && !editingAccount" class="btn-primary small" @click="handleCreateNew">添加账号</button>
        <button v-if="editingAccount" class="btn-glass btn-icon" @click="editingAccount = null">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
            <path d="M15 18l-6-6 6-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          返回列表
        </button>
        <WindowControls />
      </div>
    </div>

    <div class="settings-body">
      <Transition name="settings-fade" mode="out-in">
        <!-- 账号列表视图 -->
        <div v-if="currentTab === 'accounts' && !editingAccount" key="acc-list" class="accounts-list">
          <div v-if="accounts.length === 0" class="empty-state">
            <p>暂无账号，请点击上方按钮添加</p>
          </div>
          <div 
            v-for="acc in accounts" 
            :key="acc.id" 
            class="account-card"
            @click="editingAccount = { ...acc }; isEditing = true; originalId = acc.id"
          >
            <div class="acc-info">
              <div class="acc-avatar">{{ acc.id[0].toUpperCase() }}</div>
              <div class="acc-details">
                <div class="acc-name">{{ acc.id }}</div>
                <div class="acc-host">{{ acc.imap.host }}</div>
              </div>
            </div>
            <div class="acc-status">
              <span class="status-badge" :class="{ 
                online: acc.connected && !connectingIds.includes(acc.id),
                busy: connectingIds.includes(acc.id)
              }">
                {{ connectingIds.includes(acc.id) ? '连接中...' : (acc.connected ? '已连接' : '未连接') }}
              </span>
              <button 
                v-if="!connectingIds.includes(acc.id)"
                class="btn-icon-only" 
                title="重连"
                @click.stop="$emit('reconnect', acc.id)"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                  <path d="M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </button>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                <path d="M9 18l6-6-6-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </div>
          </div>
        </div>

        <!-- 账号编辑/新增视图 -->
        <div v-else-if="currentTab === 'accounts' && editingAccount" key="acc-form" class="account-form">
          <div class="settings-section">
            <h4 class="section-title">快捷配置预设</h4>
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

          <div class="settings-section">
            <h4 class="section-title">IMAP 收件配置</h4>
            <div class="field-group">
              <div class="field">
                <label>用户名 / 邮箱</label>
                <input v-model="editingAccount.imap.user" placeholder="your@email.com" />
              </div>
              <div class="field-row">
                <div class="field">
                  <label>服务器地址</label>
                  <input v-model="editingAccount.imap.host" placeholder="imap.example.com" />
                </div>
                <div class="field small">
                  <label>端口</label>
                  <input v-model.number="editingAccount.imap.port" type="number" />
                </div>
                <div class="field small">
                  <label>安全连接</label>
                  <GlassSelect 
                    v-model="editingAccount.imap.secure"
                    :options="secureOptions"
                  />
                </div>
              </div>
              <div class="field">
                <label>密码 / 授权码</label>
                <div class="input-with-icon">
                  <input :type="showImapPass ? 'text' : 'password'" v-model="editingAccount.imap.pass" placeholder="密码或授权码" />
                  <button class="eye-btn" @click="showImapPass = !showImapPass">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" v-if="showImapPass">
                      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" stroke="currentColor" stroke-width="2"/>
                      <circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="2"/>
                    </svg>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" v-else>
                      <path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 011.82-3.22M1 1l22 22M6.78 6.78A10.07 10.07 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19M15 15a3 3 0 11-4.24-4.24" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div class="settings-section">
            <h4 class="section-title">
              SMTP 发件配置
              <button class="btn-text" style="font-size: 10px; margin-left: auto;" @click="syncSmtp">同 IMAP</button>
            </h4>
            <div class="field-group">
              <div class="field-row">
                <div class="field">
                  <label>服务器地址</label>
                  <input v-model="editingAccount.smtp.host" placeholder="smtp.example.com" />
                </div>
                <div class="field small">
                  <label>端口</label>
                  <input v-model.number="editingAccount.smtp.port" type="number" />
                </div>
                <div class="field small">
                  <label>安全连接</label>
                  <GlassSelect 
                    v-model="editingAccount.smtp.secure"
                    :options="secureOptions"
                  />
                </div>
              </div>
              <div class="field">
                <label>用户名</label>
                <input v-model="editingAccount.smtp.user" placeholder="发件邮箱" />
              </div>
              <div class="field">
                <label>密码 / 授权码</label>
                <div class="input-with-icon">
                  <input :type="showSmtpPass ? 'text' : 'password'" v-model="editingAccount.smtp.pass" placeholder="授权码" />
                  <button class="eye-btn" @click="showSmtpPass = !showSmtpPass">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" v-if="showSmtpPass">
                      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" stroke="currentColor" stroke-width="2"/>
                      <circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="2"/>
                    </svg>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" v-else>
                      <path d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 011.82-3.22M1 1l22 22M6.78 6.78A10.07 10.07 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19M15 15a3 3 0 11-4.24-4.24" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div class="settings-actions">
            <button class="btn-primary" :disabled="connectingIds.includes(editingAccount.id)" @click="handleAction">
              {{ connectingIds.includes(editingAccount.id) ? '正在连接...' : (!isEditing ? '添加并连接' : '保存并重连') }}
            </button>
            <button v-if="isEditing" class="btn-glass danger-text" @click="handleDelete">删除账号</button>
          </div>
        </div>

        <!-- 通用设置视图 -->
        <div v-else-if="currentTab === 'general'" key="general" class="general-settings">
          <div class="settings-section">
            <h4 class="section-title">自动化与通知</h4>
            <div class="field-group">
              <div class="field">
                <label>邮件自动检查间隔 (分钟)</label>
                <div class="input-with-hint">
                  <input 
                    type="number" 
                    v-model.number="localInterval" 
                    min="1" 
                    max="1440"
                    @change="$emit('updateInterval', localInterval)"
                  />
                  <span class="hint">建议设定在 1-10 分钟之间</span>
                </div>
              </div>

              <div class="field">
                <label>界面整体缩放 ({{ zoomLevel }}%)</label>
                <div class="input-with-hint">
                  <div class="slider-row">
                    <input 
                      type="range" 
                      :value="zoomLevel" 
                      min="25" 
                      max="400" 
                      step="5"
                      @input="$emit('updateZoom', parseInt($event.target.value))"
                      class="zoom-slider"
                    />
                    <button class="btn-text" @click="$emit('updateZoom', 100)">重置</button>
                  </div>
                  <span class="hint">调整界面整体显示大小 (25% - 400%)，重置为 100% 恢复默认</span>
                </div>
              </div>

              <div class="field checkbox-field">
                <div class="checkbox-container" @click="$emit('updateAutoStart', !autoStart)">
                  <div class="custom-checkbox" :class="{ checked: autoStart }">
                    <svg v-if="autoStart" width="10" height="10" viewBox="0 0 24 24" fill="none">
                      <path d="M20 6L9 17l-5-5" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                  </div>
                  <div class="checkbox-label">
                    <span>开机自启动</span>
                    <span class="hint">开启后，应用将在系统启动时自动运行</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import GlassSelect from './GlassSelect.vue'
import WindowControls from './WindowControls.vue'

const secureOptions = [
  { label: 'SSL / TLS', value: true },
  { label: 'STARTTLS / 无', value: false }
]

const props = defineProps({
  accounts: Array,
  pollingInterval: Number,
  zoomLevel: Number,
  autoStart: Boolean,
  connectingIds: {
    type: Array,
    default: () => []
  }
})

const emit = defineEmits(['addAccount', 'deleteAccount', 'updateAccount', 'updateInterval', 'updateZoom', 'updateAutoStart', 'reconnect'])

const currentTab = ref('accounts')
const editingAccount = ref(null)
const isEditing = ref(false)
const localInterval = ref(props.pollingInterval)
const showImapPass = ref(false)
const showSmtpPass = ref(false)
let originalId = ''


function handleCreateNew() {
  isEditing.value = false
  editingAccount.value = {
    id: '',
    imap: { host: '', port: 993, secure: true, user: '', pass: '' },
    smtp: { host: '', port: 465, secure: true, user: '', pass: '' },
    connected: false
  }
}

function handleAction() {
  if (!editingAccount.value.imap.user) return
  editingAccount.value.id = editingAccount.value.imap.user
  if (!isEditing.value) {
    emit('addAccount', JSON.parse(JSON.stringify(editingAccount.value)))
  } else {
    emit('updateAccount', originalId, JSON.parse(JSON.stringify(editingAccount.value)))
  }
  editingAccount.value = null
}

function handleDelete() {
  if (confirm('确定要删除个账号吗？此操作不可撤销。')) {
    emit('deleteAccount', editingAccount.value.id)
    editingAccount.value = null
  }
}

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
  if (!editingAccount.value) return
  editingAccount.value.imap.host = preset.imap.host
  editingAccount.value.imap.port = preset.imap.port
  editingAccount.value.imap.secure = preset.imap.secure
  editingAccount.value.smtp.host = preset.smtp.host
  editingAccount.value.smtp.port = preset.smtp.port
  editingAccount.value.smtp.secure = preset.smtp.secure
}

function syncSmtp() {
  if (!editingAccount.value) return
  editingAccount.value.smtp.user = editingAccount.value.imap.user
  editingAccount.value.smtp.pass = editingAccount.value.imap.pass
}
</script>

<style scoped>
.settings-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  height: 100%;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0 12px 24px;
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
}

.header-right-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.settings-tabs {
  display: flex;
  gap: 16px;
}

.tab-btn {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-tertiary);
  padding: 8px 12px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  transition: all var(--transition-fast);
  cursor: pointer;
}

.tab-btn:hover {
  color: var(--text-secondary);
}

.tab-btn.active {
  color: var(--primary-light);
  border-bottom-color: var(--primary-light);
}

:root[data-theme="light"] .tab-btn.active {
  color: var(--primary);
  border-bottom-color: var(--primary);
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

.accounts-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.account-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.account-card:hover {
  background: var(--glass-bg-hover);
  border-color: var(--primary);
  transform: translateY(-2px);
}

.acc-info {
  display: flex;
  align-items: center;
  gap: 16px;
}

.acc-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: var(--primary);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 18px;
}

.acc-details {
  display: flex;
  flex-direction: column;
}

.acc-name {
  font-weight: 600;
  color: var(--text-primary);
  font-size: 15px;
}

.acc-host {
  font-size: 12px;
  color: var(--text-tertiary);
}

.acc-status {
  display: flex;
  align-items: center;
  gap: 12px;
  color: var(--text-tertiary);
}

.status-badge {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  background: var(--glass-border);
  color: var(--text-secondary);
}

.status-badge.online {
  background: rgba(0, 184, 148, 0.15);
  color: var(--success);
}

.status-badge.busy {
  background: rgba(253, 203, 110, 0.15);
  color: var(--warning);
}

.input-with-icon {
  position: relative;
  display: flex;
  align-items: center;
}

.input-with-icon input {
  flex: 1;
  padding-right: 40px !important;
}

.eye-btn {
  position: absolute;
  right: 10px;
  background: none;
  border: none;
  padding: 4px;
  color: var(--text-tertiary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.2s;
}

.eye-btn:hover {
  color: var(--primary-light);
}

.btn-icon-only {
  background: none;
  border: none;
  padding: 4px;
  color: var(--text-tertiary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: all 0.2s;
}

.btn-icon-only:hover {
  background: var(--glass-bg-hover);
  color: var(--primary-light);
}

.preset-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
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
  background: var(--glass-bg-hover);
  border-color: var(--primary);
}

.field-group {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.field-row {
  display: flex;
  gap: 12px;
}

.field {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
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
  padding: 10px 12px;
  background: var(--input-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 13px;
  transition: border-color var(--transition-fast);
}

.field input:focus {
  border-color: var(--primary);
  outline: none;
}

.input-with-hint {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.hint {
  font-size: 11px;
  color: var(--text-tertiary);
}

.slider-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.zoom-slider {
  flex: 1;
  -webkit-appearance: none;
  background: var(--glass-bg);
  height: 6px;
  border-radius: 3px;
  outline: none;
}

.zoom-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 16px;
  height: 16px;
  background: var(--primary);
  border-radius: 50%;
  cursor: pointer;
  border: 2px solid white;
  box-shadow: 0 2px 6px rgba(0,0,0,0.2);
}

.checkbox-field {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--glass-border);
}

.checkbox-container {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  cursor: pointer;
}

.custom-checkbox {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: 2px solid var(--glass-border-light);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  flex-shrink: 0;
  margin-top: 2px;
}

.custom-checkbox.checked {
  background: var(--primary);
  border-color: var(--primary);
  color: white;
}

.checkbox-label {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.checkbox-label span:first-child {
  font-size: 13px;
  color: var(--text-primary);
}

.settings-actions {
  display: flex;
  gap: 12px;
  padding-top: 12px;
}

.btn-text {
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  color: var(--primary-light);
  padding: 4px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
}

.btn-text:hover {
  background: var(--glass-bg-hover);
  border-color: var(--primary);
}

.btn-icon {
  display: flex;
  align-items: center;
  gap: 6px;
}

.empty-state {
  padding: 40px;
  text-align: center;
  color: var(--text-tertiary);
  background: var(--glass-bg);
  border: 1px dashed var(--glass-border);
  border-radius: var(--radius-lg);
}

.danger-text {
  color: var(--danger) !important;
}
</style>
