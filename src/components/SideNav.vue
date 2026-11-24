<template>
  <div class="sidenav">
    <div class="account-switcher" ref="menuRef">
      <!-- 账号切换触发器 -->
      <button
        class="account-trigger"
        :class="{ active: menuOpen || currentAccountId !== 'unified' }"
        title="账号列表"
        @click="menuOpen = !menuOpen"
      >
        <div class="account-icon">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
            <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.5"/>
            <path d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <circle v-if="currentAccountId === 'unified'" cx="18" cy="18" r="4" fill="var(--primary)" stroke="var(--bg-sidenav)" stroke-width="2"/>
          </svg>
          <span v-if="currentAccountId !== 'unified'" class="trigger-status" aria-hidden="true"></span>
        </div>
        <div class="arrow-indicator" :class="{ open: menuOpen }">
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none">
            <path d="M6 9l6 6 6-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
      </button>

      <!-- 账号选择弹出菜单 -->
      <transition name="pop-in">
        <div v-if="menuOpen" class="account-popup">
          <div class="popup-header">选择账号</div>
          <div class="popup-list">
            <div 
              class="popup-item" 
              :class="{ active: currentAccountId === 'unified' }"
              @click="selectAccount('unified')"
            >
              <div class="item-icon">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                  <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.2"/>
                  <path d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </div>
              <span class="item-label">所有收件箱</span>
              <span v-if="unreadTotal > 0" class="unread-count-badge">{{ formatUnread(unreadTotal) }}</span>
              <div v-if="currentAccountId === 'unified'" class="check-mark">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none">
                  <path d="M20 6L9 17L4 12" stroke="var(--primary)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </div>
            </div>
            
            <div class="divider"></div>

            <div 
              v-for="acc in accounts" 
              :key="acc.id" 
              class="popup-item"
              :class="{ active: currentAccountId === acc.id }"
              @click="selectAccount(acc.id)"
            >
              <div class="item-avatar">{{ getInitial(acc.id) }}</div>
              <div class="item-info">
                <div class="item-label">{{ acc.id }}</div>
                <div class="item-status" :class="{ online: acc.connected }">
                  {{ acc.enabled === false ? '已停用' : (acc.connected ? '已连接' : '已离线') }}
                </div>
              </div>
              <span v-if="acc.enabled !== false && acc.unreadCount > 0" class="unread-count-badge">{{ formatUnread(acc.unreadCount) }}</span>
              <div v-if="currentAccountId === acc.id" class="check-mark">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none">
                  <path d="M20 6L9 17L4 12" stroke="var(--primary)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </div>
            </div>
          </div>
        </div>
      </transition>
    </div>

    <nav class="sidenav-items">
      <span
        v-if="activeNavIndex >= 0"
        class="nav-selection-indicator"
        :style="{ top: `${16 + activeNavIndex * 46}px` }"
        aria-hidden="true"
      ></span>
      <button
        v-for="item in navItems"
        :key="item.id"
        class="nav-item"
        :class="{ active: activeView === item.id }"
        :title="item.label"
        @click="handleClick(item)"
      >
        <div class="nav-icon" v-html="item.icon"></div>
        <span v-if="item.id === 'inbox' && unreadTotal > 0" class="nav-unread-badge">{{ formatUnread(unreadTotal) }}</span>
      </button>
    </nav>

    <div class="sidenav-bottom">
      <!-- 深浅色模式切换 -->
      <button
        class="nav-item"
        :title="isDark ? '切换浅色模式' : '切换深色模式'"
        @click="toggleTheme"
      >
        <div class="nav-icon">
          <svg v-if="isDark" width="20" height="20" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="5" stroke="currentColor" stroke-width="1.5"/>
            <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          <svg v-else width="20" height="20" viewBox="0 0 24 24" fill="none">
            <path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
      </button>

      <!-- 设置 -->
      <button
        class="nav-item"
        :class="{ active: activeView === 'settings' }"
        title="设置"
        @click="$emit('navigate', 'settings')"
      >
        <div class="nav-icon">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
            <path d="M12 15a3 3 0 100-6 3 3 0 000 6z" stroke="currentColor" stroke-width="1.5"/>
            <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z" stroke="currentColor" stroke-width="1.5"/>
          </svg>
        </div>
      </button>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'

const props = defineProps({
  activeView: String,
  accounts: Array,
  unreadTotal: { type: Number, default: 0 },
  currentAccountId: String,
})

const emit = defineEmits(['navigate', 'switchFolder', 'switchAccount'])

function getInitial(id) {
  return id ? id[0].toUpperCase() : '?'
}

function formatUnread(count) {
  return count > 99 ? '99+' : String(count)
}

const isDark = ref(true)

const menuOpen = ref(false)
const menuRef = ref(null)

onMounted(() => {
  const saved = localStorage.getItem('4ymail-theme')
  if (saved === 'light') {
    isDark.value = false
    document.documentElement.setAttribute('data-theme', 'light')
  }

  // 点击外部关闭菜单
  document.addEventListener('click', (e) => {
    if (menuRef.value && !menuRef.value.contains(e.target)) {
      menuOpen.value = false
    }
  })
})

function selectAccount(id) {
  emit('switchAccount', id)
  menuOpen.value = false
}

function toggleTheme() {
  isDark.value = !isDark.value
  if (isDark.value) {
    document.documentElement.removeAttribute('data-theme')
    localStorage.setItem('4ymail-theme', 'dark')
  } else {
    document.documentElement.setAttribute('data-theme', 'light')
    localStorage.setItem('4ymail-theme', 'light')
  }
}

function handleClick(item) {
  if (item.folder) {
    emit('switchFolder', item.folder)
  }
  emit('navigate', item.id)
}

const navItems = [
  {
    id: 'inbox',
    label: '收件箱',
    folder: 'INBOX',
    icon: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none">
      <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.5"/>
      <path d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
  },
  {
    id: 'sent',
    label: '已发送',
    folder: 'Sent Messages',
    icon: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none">
      <path d="M22 2L11 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      <path d="M22 2L15 22l-4-9-9-4L22 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
    </svg>`,
  },
  {
    id: 'drafts',
    label: '草稿箱',
    folder: 'Drafts',
    icon: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M14 2v6h6M16 13H8M16 17H8M10 9H8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
    </svg>`,
  },
  {
    id: 'trash',
    label: '垃圾箱',
    folder: 'Deleted Messages',
    icon: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none">
      <path d="M3 6h18M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      <path d="M10 11v6M14 11v6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
    </svg>`,
  },
  {
    id: 'compose',
    label: '写邮件',
    icon: `<svg width="20" height="20" viewBox="0 0 24 24" fill="none">
      <path d="M12 20h9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      <path d="M16.5 3.5a2.121 2.121 0 013 3L7 19l-4 1 1-4L16.5 3.5z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
  },
]

const activeNavIndex = computed(() => navItems.findIndex(item => item.id === props.activeView))
</script>

<style scoped>
.sidenav {
  width: var(--sidebar-width);
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  background: var(--bg-sidenav);
  border-right: 1px solid var(--glass-border);
  padding: 12px 0;
  flex-shrink: 0;
  transition: background var(--transition-normal);
}

.sidenav-items {
  position: relative;
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 4px 0;
}

.nav-selection-indicator {
  position: absolute;
  z-index: 1;
  left: 2px;
  width: 3px;
  height: 20px;
  border-radius: 0 3px 3px 0;
  background: var(--primary-light);
  pointer-events: none;
  transition: top 0.3s cubic-bezier(0.22, 0.61, 0.36, 1), background var(--transition-normal);
}

:root[data-theme="light"] .nav-selection-indicator {
  background: var(--primary);
}

.account-switcher {
  position: relative;
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-bottom: 12px;
}

.account-trigger {
  position: relative;
  width: 44px;
  height: 44px;
  border-radius: var(--radius-md);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.account-trigger:hover {
  background: var(--glass-bg-hover);
  border-color: var(--primary);
}

.account-trigger.active {
  background: var(--glass-bg-active);
  border-color: var(--primary);
  color: var(--primary-light);
}

.trigger-status {
  position: absolute;
  right: 3px;
  bottom: 3px;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--primary-light);
  border: 1.5px solid var(--bg-sidenav);
  box-shadow: 0 0 0 2px rgba(0, 184, 148, 0.14);
  animation: account-indicator-in 0.2s ease-out both;
}

@keyframes account-indicator-in {
  from { opacity: 0; transform: scale(0.65); }
  to { opacity: 1; transform: scale(1); }
}

.arrow-indicator {
  position: absolute;
  bottom: 0px;
  opacity: 0.5;
  transition: transform var(--transition-fast);
}

.arrow-indicator.open {
  transform: rotate(180deg);
}

/* 弹出菜单 */
.account-popup {
  position: absolute;
  top: 0;
  left: 56px;
  width: 240px;
  background: var(--bg-sidenav);
  backdrop-filter: blur(10px);
  border: 1px solid var(--glass-border-light);
  border-radius: var(--radius-lg);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  z-index: 1000;
  padding: 8px;
  overflow: hidden;
}

:root[data-theme="light"] .account-popup {
  background: rgba(255, 255, 255, 0.95);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
}

.popup-header {
  padding: 8px 12px;
  font-size: 11px;
  font-weight: 700;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.popup-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.popup-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.popup-item:hover {
  background: var(--glass-bg-hover);
}

.popup-item.active {
  background: rgba(0, 184, 148, 0.12);
}

.item-icon, .item-avatar {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  background: var(--glass-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--text-secondary);
}

.item-avatar {
  background: rgba(0, 184, 148, 0.12);
  color: var(--primary-light);
  font-weight: 600;
  font-size: 12px;
  transition: background var(--transition-fast), color var(--transition-fast), transform var(--transition-fast);
}

.popup-item.active .item-avatar {
  background: var(--primary);
  color: white;
  transform: scale(1.04);
}

.item-info {
  flex: 1;
  min-width: 0;
}

.item-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.item-status {
  font-size: 10px;
  color: var(--text-tertiary);
}

.item-status.online {
  color: var(--success);
}

.unread-count-badge,
.nav-unread-badge {
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 999px;
  background: var(--danger);
  color: white;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: 700;
  line-height: 1;
  flex-shrink: 0;
}

.check-mark {
  color: var(--primary);
}

.divider {
  height: 1px;
  background: var(--glass-border);
  margin: 4px 0;
}

/* 动画 */
.pop-in-enter-active, .pop-in-leave-active {
  transition: opacity 0.18s ease, transform 0.22s cubic-bezier(0.22, 0.61, 0.36, 1);
}

.pop-in-enter-from {
  opacity: 0;
  transform: translateX(-6px) scale(0.985);
}

.pop-in-leave-to {
  opacity: 0;
  transform: translateX(-3px) scale(0.99);
}

.sidenav-bottom {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding-top: 8px;
  border-top: 1px solid var(--glass-border);
}

.nav-item {
  position: relative;
  width: 44px;
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-secondary);
  transition: all var(--transition-fast);
}

.nav-item::before {
  content: '';
  position: absolute;
  left: -8px;
  width: 3px;
  height: 20px;
  border-radius: 0 3px 3px 0;
  background: var(--primary-light);
  opacity: 0;
  transform: scaleY(0.65);
  transition: opacity 0.18s ease, transform 0.24s cubic-bezier(0.22, 0.61, 0.36, 1);
}

.nav-item:hover {
  background: var(--glass-bg-hover);
  color: var(--text-primary);
}

.nav-item.active {
  background: rgba(0, 184, 148, 0.15);
  color: var(--primary-light);
}

:root[data-theme="light"] .nav-item.active {
  background: rgba(9, 132, 227, 0.12);
  color: var(--primary);
}

.nav-item.active::before {
  opacity: 1;
  transform: scaleY(1);
}

.sidenav-items .nav-item::before {
  display: none;
}

.nav-unread-badge {
  position: absolute;
  top: 2px;
  right: 0;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  font-size: 9px;
}

:root[data-theme="light"] .nav-item.active::before {
  background: var(--primary);
}

.nav-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.2s cubic-bezier(0.22, 0.61, 0.36, 1);
}

.nav-item.active .nav-icon {
  transform: scale(1.04);
}

.nav-item:active .nav-icon {
  transform: scale(0.94);
}

.check-mark {
  animation: selection-check-in 0.18s cubic-bezier(0.16, 1, 0.3, 1) both;
}

@keyframes selection-check-in {
  from { opacity: 0; transform: scale(0.7); }
  to { opacity: 1; transform: scale(1); }
}
</style>
