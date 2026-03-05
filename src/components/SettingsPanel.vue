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
      <button v-if="currentTab === 'accounts' && !editingAccount" class="btn-primary small" @click="handleCreateNew">添加账号</button>
      <button v-if="editingAccount" class="btn-glass btn-icon" @click="editingAccount = null">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
          <path d="M15 18l-6-6 6-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        返回列表
      </button>
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
            @click="editingAccount = { ...acc }"
          >
            <div class="acc-info">
              <div class="acc-avatar">{{ acc.id[0].toUpperCase() }}</div>
              <div class="acc-details">
                <div class="acc-name">{{ acc.id }}</div>
                <div class="acc-host">{{ acc.imap.host }}</div>
              </div>
            </div>
            <div class="acc-status">
              <span class="status-badge" :class="{ online: acc.connected }">
                {{ acc.connected ? '已连接' : '未连接' }}
              </span>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                <path d="M9 18l6-6-6-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </div>
          </div>
        </div>

        <!-- 账号编辑/新增视图 -->
        <div v-else-if="currentTab === 'accounts' && editingAccount" key="acc-form" class="account-form">
          <!-- 快捷预设 -->
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
                  <select v-model="editingAccount.imap.secure" class="select-field">
                    <option :value="true">SSL / TLS</option>
                    <option :value="false">STARTTLS / 无</option>
                  </select>
                </div>
              </div>
              <div class="field">
                <label>密码 / 授权码</label>
                <input v-model="editingAccount.imap.pass" type="password" placeholder="密码或授权码" />
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
                  <select v-model="editingAccount.smtp.secure" class="select-field">
                    <option :value="true">SSL / TLS</option>
                    <option :value="false">STARTTLS / 无</option>
                  </select>
                </div>
              </div>
              <div class="field">
                <label>用户名</label>
                <input v-model="editingAccount.smtp.user" placeholder="发件邮箱" />
              </div>
              <div class="field">
                <label>密码 / 授权码</label>
                <input v-model="editingAccount.smtp.pass" type="password" placeholder="授权码" />
              </div>
            </div>
          </div>

          <div class="settings-actions">
            <button class="btn-primary" @click="handleAction">{{ isNew ? '添加并连接' : '保存修改' }}</button>
            <button v-if="!isNew" class="btn-glass danger-text" @click="handleDelete">删除账号</button>
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
            </div>
          </div>
        </div>
      </Transition>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'

const props = defineProps({
  accounts: Array,
  pollingInterval: Number,
})

const emit = defineEmits(['addAccount', 'deleteAccount', 'updateAccount', 'updateInterval'])

const currentTab = ref('accounts')
const editingAccount = ref(null)
const localInterval = ref(props.pollingInterval)

const isNew = computed(() => {
  if (!editingAccount.value) return false
  return !props.accounts.find(a => a.id === editingAccount.value.id)
})

function handleCreateNew() {
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
  if (isNew.value) {
    emit('addAccount', JSON.parse(JSON.stringify(editingAccount.value)))
  } else {
    emit('updateAccount', JSON.parse(JSON.stringify(editingAccount.value)))
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

/* 账号列表 */
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

/* 输入框提示 */
.input-with-hint {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.hint {
  font-size: 11px;
  color: var(--text-tertiary);
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

/* 原有样式保持 */
.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 24px;
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
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

.btn-text {
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  color: var(--primary-light);
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-text:hover {
  background: var(--glass-bg-hover);
  border-color: var(--primary);
}

.select-field {
  padding: 8px 10px;
  background: var(--input-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
}

.select-field option {
  background: var(--bg-card);
  color: var(--text-primary);
}

.settings-actions {
  display: flex;
  gap: 10px;
  padding-top: 8px;
}

.danger-text {
  color: var(--danger) !important;
}
</style>
