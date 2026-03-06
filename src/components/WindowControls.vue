<template>
  <div class="window-controls">
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
.window-controls {
  display: flex;
  -webkit-app-region: no-drag;
  z-index: 1000;
}

.ctrl-btn {
  width: 42px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--text-secondary);
  border: none;
  transition: all var(--transition-fast);
  cursor: pointer;
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
  transform: scale(0.9);
}
</style>
