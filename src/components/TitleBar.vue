<template>
  <div class="titlebar" @dblclick="toggleMaximize">
    <div class="titlebar-drag">
      <div class="titlebar-logo">
        <img src="/logo.png" alt="4yMail" class="titlebar-icon" />
        <span class="titlebar-title">4yMail</span>
      </div>
    </div>
    <div class="titlebar-controls">
      <button class="ctrl-btn minimize" @click="minimize" title="最小化">
        <svg width="12" height="12" viewBox="0 0 12 12"><path d="M2 6h8" stroke="currentColor" stroke-width="1.2"/></svg>
      </button>
      <button class="ctrl-btn maximize" @click="toggleMaximize" title="最大化">
        <svg width="12" height="12" viewBox="0 0 12 12"><rect x="2" y="2" width="8" height="8" rx="1" stroke="currentColor" stroke-width="1.2" fill="none"/></svg>
      </button>
      <button class="ctrl-btn close" @click="closeWin" title="关闭">
        <svg width="12" height="12" viewBox="0 0 12 12"><path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.2"/></svg>
      </button>
    </div>
  </div>
</template>

<script setup>
const isElectron = typeof window !== 'undefined' && window.electronAPI

function minimize() {
  if (isElectron) window.electronAPI.minimize()
}
function toggleMaximize() {
  if (isElectron) window.electronAPI.maximize()
}
function closeWin() {
  if (isElectron) window.electronAPI.close()
}
</script>

<style scoped>
.titlebar {
  height: var(--titlebar-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-titlebar);
  border-bottom: 1px solid var(--glass-border);
  -webkit-app-region: drag;
  flex-shrink: 0;
  transition: background var(--transition-normal);
}

.titlebar-drag {
  flex: 1;
  display: flex;
  align-items: center;
  padding-left: 14px;
}

.titlebar-logo {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-secondary);
}

.titlebar-icon {
  width: 18px;
  height: 18px;
  object-fit: contain;
}

.titlebar-title {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.5px;
}

.titlebar-controls {
  display: flex;
  -webkit-app-region: no-drag;
}

.ctrl-btn {
  width: 46px;
  height: var(--titlebar-height);
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--text-secondary);
  border: none;
  transition: all var(--transition-fast);
}

.ctrl-btn:hover {
  background: var(--glass-bg-hover);
  color: var(--text-primary);
}

.ctrl-btn.close:hover {
  background: var(--ctrl-close-hover);
  color: white;
}

.ctrl-btn:active {
  transform: none;
}
</style>
