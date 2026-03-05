<template>
  <div class="maillist">
    <div class="maillist-header">
      <div class="search-box">
        <svg class="search-icon" width="15" height="15" viewBox="0 0 24 24" fill="none">
          <circle cx="11" cy="11" r="8" stroke="currentColor" stroke-width="1.5"/>
          <path d="M21 21l-4.35-4.35" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索邮件..."
          class="search-input"
        />
      </div>
      <div class="maillist-toolbar">
        <select
          v-if="folders.length"
          class="folder-select"
          :value="currentFolder"
          @change="$emit('changeFolder', ($event.target).value)"
        >
          <option v-for="f in folders" :key="f.path" :value="f.path">
            {{ getFolderName(f) }}
          </option>
        </select>
        <button class="toolbar-btn" @click="$emit('refresh')" title="刷新">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" :class="{ spinning: loading }">
            <path d="M23 4v6h-6M1 20v-6h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </div>
    </div>

    <div class="maillist-body" ref="listRef">
      <div v-if="loading && filteredMails.length === 0" class="maillist-empty">
        <div class="loading-spinner"></div>
        <span>正在加载...</span>
      </div>
      <div v-else-if="filteredMails.length === 0" class="maillist-empty">
        <svg width="40" height="40" viewBox="0 0 24 24" fill="none" style="opacity:0.3">
          <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.5"/>
          <path d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1.5"/>
        </svg>
        <span>没有邮件</span>
      </div>
      <TransitionGroup name="list" tag="div" class="mail-items-container">
        <div
          v-for="(mail, index) in filteredMails"
          :key="mail.uid"
          class="mail-item stagger-item"
          :class="{ selected: selectedMail === mail.uid, unread: !mail.seen }"
          :style="{ '--i': index }"
          @click="$emit('select', mail)"
          @contextmenu.prevent="openContext($event, mail)"
        >
          <div class="mail-avatar" :style="{ background: getAvatarColor(mail.from.address) }">
            {{ getInitial(mail.from.name) }}
          </div>
          <div class="mail-info">
            <div class="mail-top">
              <span class="mail-sender">{{ mail.from.name || mail.from.address }}</span>
              <span class="mail-time">{{ formatTime(mail.date) }}</span>
            </div>
            <div class="mail-subject" :class="{ 'font-bold': !mail.seen }">{{ mail.subject }}</div>
            <div v-if="mail.accountId" class="acc-label">{{ mail.accountId }}</div>
          </div>
          <div v-if="!mail.seen" class="unread-dot"></div>
        </div>
      </TransitionGroup>
    </div>

    <!-- 右键菜单 -->
    <teleport to="body">
      <transition name="ctx-pop">
        <div
          v-if="showContext"
          class="context-menu"
          :style="{ left: contextPos.x + 'px', top: contextPos.y + 'px' }"
          ref="ctxMenuRef"
        >
          <div class="ctx-item" @click="ctxAction('toggleRead')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path v-if="contextMail?.seen" d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" stroke="currentColor" stroke-width="1.5"/>
              <circle v-if="contextMail?.seen" cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.5"/>
              <path v-if="!contextMail?.seen" d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.5"/>
              <path v-if="!contextMail?.seen" d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <span>{{ contextMail?.seen ? '标记为未读' : '标记为已读' }}</span>
          </div>
          <div class="ctx-item" @click="ctxAction('reply')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path d="M9 17l-5-5 5-5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M4 12h12a4 4 0 014 4v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <span>回复</span>
          </div>
          <div class="ctx-divider"></div>
          <div class="ctx-item" @click="ctxAction('moveTrash')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path d="M3 6h18M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>移至垃圾箱</span>
          </div>
          <div class="ctx-item danger" @click="ctxAction('delete')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
            <span>彻底删除</span>
          </div>
          <div class="ctx-divider"></div>
          <div class="ctx-item" @click="ctxAction('copyAddr')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="1.5"/>
              <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            <span>复制发件人地址</span>
          </div>
        </div>
      </transition>
    </teleport>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'

const props = defineProps({
  mails: Array,
  selectedMail: [Number, String, null],
  currentFolder: String,
  loading: Boolean,
  folders: Array,
})

const emit = defineEmits(['select', 'refresh', 'changeFolder', 'markRead', 'markUnread', 'reply', 'delete', 'moveTrash', 'copyAddr'])

const searchQuery = ref('')
const showContext = ref(false)
const contextMail = ref(null)
const contextPos = ref({ x: 0, y: 0 })
const ctxMenuRef = ref(null)

function openContext(event, mail) {
  // 先关闭已有菜单
  showContext.value = false
  contextMail.value = mail
  // 确保菜单不超出屏幕
  const menuW = 200, menuH = 260
  let x = event.clientX
  let y = event.clientY
  if (x + menuW > window.innerWidth) x = window.innerWidth - menuW - 8
  if (y + menuH > window.innerHeight) y = window.innerHeight - menuH - 8
  contextPos.value = { x, y }
  // 使用 nextTick 延迟，避免与全局 click 事件冲突
  requestAnimationFrame(() => {
    showContext.value = true
  })
}

function ctxAction(action) {
  if (!contextMail.value) return
  const mail = contextMail.value
  switch (action) {
    case 'toggleRead':
      if (mail.seen) {
        emit('markUnread', mail)
      } else {
        emit('markRead', mail)
      }
      break
    case 'reply':
      emit('select', mail)
      // 稍微延迟以确保邮件先加载
      setTimeout(() => emit('reply', mail), 100)
      break
    case 'delete':
      emit('delete', mail)
      break
    case 'moveTrash':
      emit('moveTrash', mail)
      break
    case 'copyAddr':
      emit('copyAddr', mail)
      break
  }
  showContext.value = false
}

function closeContext(e) {
  if (showContext.value && ctxMenuRef.value && !ctxMenuRef.value.contains(e.target)) {
    showContext.value = false
  }
}

onMounted(() => {
  document.addEventListener('mousedown', closeContext)
})

onUnmounted(() => {
  document.removeEventListener('mousedown', closeContext)
})


const filteredMails = computed(() => {
  if (!searchQuery.value.trim()) return props.mails || []
  const q = searchQuery.value.toLowerCase()
  return (props.mails || []).filter(m =>
    m.subject.toLowerCase().includes(q) ||
    m.from.name.toLowerCase().includes(q) ||
    m.from.address.toLowerCase().includes(q)
  )
})

function getFolderName(folder) {
  const map = {
    '\\Inbox': '收件箱',
    '\\Sent': '已发送',
    '\\Drafts': '草稿箱',
    '\\Trash': '垃圾箱',
    '\\Junk': '垃圾邮件',
    '\\Archive': '归档',
  }
  if (folder.specialUse && map[folder.specialUse]) return map[folder.specialUse]
  return folder.name
}

function getAvatarColor(email) {
  const colors = [
    'linear-gradient(135deg, #6c5ce7, #a29bfe)',
    'linear-gradient(135deg, #00cec9, #55efc4)',
    'linear-gradient(135deg, #fd79a8, #e84393)',
    'linear-gradient(135deg, #fdcb6e, #f39c12)',
    'linear-gradient(135deg, #74b9ff, #0984e3)',
    'linear-gradient(135deg, #55efc4, #00b894)',
    'linear-gradient(135deg, #fab1a0, #e17055)',
    'linear-gradient(135deg, #dfe6e9, #b2bec3)',
  ]
  let hash = 0
  for (let i = 0; i < (email || '').length; i++) {
    hash = email.charCodeAt(i) + ((hash << 5) - hash)
  }
  return colors[Math.abs(hash) % colors.length]
}

function getInitial(name) {
  if (!name) return '?'
  // 中文名取最后一个字，英文取首字母
  const trimmed = name.trim()
  if (/[\u4e00-\u9fa5]/.test(trimmed)) {
    return trimmed.charAt(trimmed.length - 1)
  }
  return trimmed.charAt(0).toUpperCase()
}

function formatTime(dateStr) {
  if (!dateStr) return ''
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const oneDay = 86400000

  if (diff < oneDay && date.getDate() === now.getDate()) {
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  }
  if (diff < oneDay * 2) return '昨天'
  if (diff < oneDay * 7) {
    const days = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
    return days[date.getDay()]
  }
  return date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })
}
</script>

<style scoped>
.maillist {
  width: var(--maillist-width);
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-maillist);
  border-right: 1px solid var(--glass-border);
  flex-shrink: 0;
  transition: background var(--transition-normal);
}

.maillist-header {
  padding: 12px;
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
}

.search-box {
  position: relative;
  margin-bottom: 8px;
}

.search-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-tertiary);
}

.search-input {
  width: 100%;
  padding: 8px 12px 8px 34px;
  background: var(--input-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  color: var(--text-primary);
  font-size: 13px;
}

.search-input:focus {
  border-color: var(--primary);
}

.maillist-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
}

.folder-select {
  flex: 1;
  padding: 5px 8px;
  background: var(--input-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 12px;
  font-family: inherit;
  outline: none;
  cursor: pointer;
}

.folder-select option {
  background: var(--select-bg);
  color: var(--text-primary);
}

.toolbar-btn {
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
}

.toolbar-btn:hover {
  background: var(--glass-bg-hover);
  color: var(--text-primary);
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.spinning {
  animation: spin 1s linear infinite;
}

.maillist-body {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.maillist-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 200px;
  gap: 12px;
  color: var(--text-tertiary);
  font-size: 13px;
}

.loading-spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--glass-border);
  border-top-color: var(--primary-light);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.mail-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  cursor: pointer;
  transition: background var(--transition-fast);
  position: relative;
}

.mail-item:hover {
  background: var(--glass-bg-hover);
}

.mail-item.selected {
  background: rgba(0, 184, 148, 0.10);
}

.mail-item.selected::before {
  content: '';
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 3px;
  background: var(--primary-light);
  border-radius: 0 3px 3px 0;
}

.mail-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-size: 15px;
  font-weight: 600;
  flex-shrink: 0;
}

.mail-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.mail-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.mail-sender {
  font-size: 13px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 160px;
}

.mail-time {
  font-size: 11px;
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.mail-subject {
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.font-bold {
  font-weight: 600;
}

.unread-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--primary);
  flex-shrink: 0;
}

.acc-label {
  font-size: 10px;
  color: var(--text-tertiary);
  margin-top: 4px;
  opacity: 0.7;
}

:root[data-theme="light"] .acc-label {
  color: var(--primary);
}
</style>

<style>
/* 右键菜单 - 使用全局样式因为 teleport 到 body */
.context-menu {
  position: fixed;
  z-index: 10000;
  min-width: 180px;
  padding: 6px;
  background: rgba(30, 30, 50, 0.92);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
}

:root[data-theme="light"] .context-menu {
  background: rgba(255, 255, 255, 0.96);
  border-color: rgba(0, 0, 0, 0.08);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.12);
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 14px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.85);
  cursor: pointer;
  transition: background 0.12s ease;
}

:root[data-theme="light"] .ctx-item {
  color: rgba(0, 0, 0, 0.75);
}

.ctx-item:hover {
  background: rgba(255, 255, 255, 0.1);
}

:root[data-theme="light"] .ctx-item:hover {
  background: rgba(0, 0, 0, 0.06);
}

.ctx-item.danger {
  color: #ff6b6b;
}

.ctx-item.danger:hover {
  background: rgba(255, 107, 107, 0.12);
}

.ctx-divider {
  height: 1px;
  background: rgba(255, 255, 255, 0.08);
  margin: 4px 8px;
}

:root[data-theme="light"] .ctx-divider {
  background: rgba(0, 0, 0, 0.06);
}

/* 右键菜单动画 */
.ctx-pop-enter-active {
  transition: all 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.ctx-pop-leave-active {
  transition: all 0.15s ease-in;
}

.ctx-pop-enter-from {
  opacity: 0;
  transform: scale(0.8) translateY(-10px);
}

.ctx-pop-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

.mail-items-container {
  position: relative;
  width: 100%;
}
</style>
