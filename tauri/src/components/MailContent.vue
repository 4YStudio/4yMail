<template>
  <div class="mailcontent" @contextmenu.prevent="handleContextMenu">
    <WindowControls class="main-window-controls" />

    <!-- 空状态 -->
    <div v-if="!mail && !loading" class="mailcontent-empty">
      <div class="empty-icon">
        <svg width="64" height="64" viewBox="0 0 24 24" fill="none">
          <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1"/>
          <path d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1"/>
        </svg>
      </div>
      <p class="empty-text">选择一封邮件开始阅读</p>
      <button class="btn-primary compose-btn" @click="$emit('compose')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
          <path d="M12 5v14M5 12h14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        写新邮件
      </button>
    </div>

    <!-- 加载中 -->
    <div v-else-if="loading" class="mailcontent-empty">
      <div class="loading-spinner"></div>
      <span>加载中...</span>
    </div>

    <!-- 邮件详情 -->
    <template v-else-if="mail">
      <div class="mail-detail-header">
        <h2 class="mail-detail-subject">{{ mail.subject }}</h2>
        <div class="mail-detail-actions">
          <button class="action-btn" @click="$emit('reply', mail)" title="回复">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
              <path d="M9 17l-5-5 5-5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M4 12h12a4 4 0 014 4v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <button class="action-btn" title="转发">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
              <path d="M15 17l5-5-5-5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M20 12H8a4 4 0 00-4 4v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <button class="action-btn danger" @click="$emit('delete', mail.uid)" title="删除">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
              <path d="M3 6h18M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
      </div>

      <div class="mail-detail-meta">
        <div class="meta-avatar" :style="{ background: getAvatarColor(mail.from.address) }">
          {{ getInitial(mail.from.name) }}
        </div>
        <div class="meta-info">
          <div class="meta-from">
            <span class="from-name">{{ mail.from.name }}</span>
            <span class="from-addr">&lt;{{ mail.from.address }}&gt;</span>
          </div>
          <div class="meta-to">
            收件人: {{ mail.to?.map(t => t.name || t.address).join(', ') || '-' }}
          </div>
          <div v-if="mail.cc && mail.cc.length" class="meta-cc">
            抄送: {{ mail.cc.map(t => t.name || t.address).join(', ') }}
          </div>
        </div>
        <div class="meta-date">{{ formatFullDate(mail.date) }}</div>
      </div>

      <div class="mail-detail-body selectable" ref="bodyRef" @click="handleBodyClick">
        <div 
          v-if="mail.html" 
          v-html="deferredHtml" 
          class="mail-html-content"
          :class="{ 'dynamic-theme': dynamicMailTheme }"
        ></div>
        <pre v-else-if="mail.text" class="mail-text-content">{{ mail.text }}</pre>
        <p v-else class="mail-no-content">（邮件无内容）</p>
      </div>

      <div v-if="mail.attachments && mail.attachments.length" class="mail-detail-attachments">
        <h4 class="att-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
            <path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          附件 ({{ mail.attachments.length }})
        </h4>
        <div class="att-list">
          <div 
            v-for="(att, i) in mail.attachments" 
            :key="i" 
            class="att-item"
            @click="handleDownload(i, att.filename)"
            title="点击下载附件"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
              <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z" stroke="currentColor" stroke-width="1.5"/>
              <path d="M14 2v6h6" stroke="currentColor" stroke-width="1.5"/>
            </svg>
            <span class="att-name">{{ att.filename }}</span>
            <span class="att-size">{{ formatSize(att.size) }}</span>
          </div>
        </div>
      </div>
    </template>

    <!-- 右键菜单 -->
    <teleport to="body">
      <transition name="ctx-pop">
        <div
          v-if="showContext"
          class="context-menu"
          :style="{ left: contextPos.x + 'px', top: contextPos.y + 'px' }"
          ref="ctxMenuRef"
        >
          <!-- 文本操作：复制 -->
          <template v-if="contextType === 'text'">
            <div class="ctx-item" @click="ctxAction('copy')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="1.5"/>
                <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>复制</span>
            </div>
            <div class="ctx-divider"></div>
          </template>

          <!-- 链接操作 -->
          <template v-if="contextType === 'link'">
            <div class="ctx-item" @click="ctxAction('copyLink')">
               <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                 <path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                 <path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
               </svg>
               <span>复制链接</span>
            </div>
            <div class="ctx-item" @click="ctxAction('openLink')">
               <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                 <path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
               </svg>
               <span>打开链接</span>
            </div>
            <div class="ctx-divider"></div>
          </template>

          <!-- 邮件操作 -->
          <template v-if="contextType === 'mail' || contextType === 'text' || contextType === 'link'">
            <div class="ctx-item" @click="ctxAction('toggleRead')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path v-if="mail?.seen" d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" stroke="currentColor" stroke-width="1.5"/>
                <circle v-if="mail?.seen" cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.5"/>
                <path v-if="!mail?.seen" d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.5"/>
                <path v-if="!mail?.seen" d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>{{ mail?.seen ? '标记为未读' : '标记为已读' }}</span>
            </div>
            <div class="ctx-item" @click="ctxAction('reply')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M9 17l-5-5 5-5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M4 12h12a4 4 0 014 4v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>回复</span>
            </div>
            <div class="ctx-item" @click="ctxAction('forward')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M15 17l5-5-5-5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M20 12H8a4 4 0 00-4 4v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>转发</span>
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
          </template>

          <!-- 全选（仅在文本模式显示） -->
          <template v-if="contextType === 'text' || contextType === 'link'">
            <div class="ctx-divider"></div>
            <div class="ctx-item" @click="ctxAction('selectAll')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M4 4h16v16H4V4z" stroke="currentColor" stroke-width="1.5"/>
                <path d="M9 9h6v6H9V9z" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>全选</span>
            </div>
          </template>
        </div>
      </transition>
    </teleport>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, computed, watch, nextTick } from 'vue'
import WindowControls from './WindowControls.vue'

const props = defineProps({
  mail: Object,
  loading: Boolean,
  dynamicMailTheme: {
    type: Boolean,
    default: true
  }
})

const emit = defineEmits(['reply', 'delete', 'compose', 'markRead', 'markUnread', 'moveTrash', 'forward'])

// --- 性能优化：内容延迟加载与解析 ---
const deferredHtml = ref('')
let renderTimer = null

// 使用 DOMParser 进行高效清理，替代性能较差的正则
function getSanitizedHtml(html) {
  if (!html) return ''
  
  try {
    const parser = new DOMParser()
    const doc = parser.parseFromString(html, 'text/html')
    
    // 提取 body 属性
    const body = doc.body
    const bodyAttrs = Array.from(body.attributes)
      .map(attr => `${attr.name}="${attr.value}"`)
      .join(' ')
    
    // 移除脚本和插件
    const scripts = doc.querySelectorAll('script, object, embed, iframe:not(.mail-content-iframe)')
    scripts.forEach(s => s.remove())
    
    // 移除事件处理器
    const allElements = doc.querySelectorAll('*')
    allElements.forEach(el => {
      const attrs = el.attributes
      for (let i = attrs.length - 1; i >= 0; i--) {
        if (attrs[i].name.startsWith('on')) {
          el.removeAttribute(attrs[i].name)
        }
      }
    })
    
    return `<div class="mail-body-wrapper" ${bodyAttrs}>${body.innerHTML}</div>`
  } catch (e) {
    console.error('HTML sanitize failed:', e)
    return html // 降级处理
  }
}

// 监听邮件变化，执行延迟渲染
watch(() => props.mail, (newMail) => {
  if (renderTimer) clearTimeout(renderTimer)
  
  // 1. 先清空内容，防止旧内容的 DOM 操作干扰新动画
  deferredHtml.value = ''
  
  if (newMail && newMail.html) {
    // 2. 延迟渲染：等待侧滑动画 (300ms) 开始后再插入 DOM
    // 这样就不会在动画第一帧由于 DOM 插入导致掉帧
    renderTimer = setTimeout(() => {
      deferredHtml.value = getSanitizedHtml(newMail.html)
    }, 50) 
  }
}, { immediate: true })

// --- 右键菜单相关 ---
const bodyRef = ref(null)
const showContext = ref(false)
const contextPos = ref({ x: 0, y: 0 })
const ctxMenuRef = ref(null)
const selectionText = ref('')
const linkUrl = ref('')
const contextType = ref('text') // 'text' | 'mail'

function handleContextMenu(event) {
  if (!props.mail) return

  selectionText.value = window.getSelection().toString().trim()
  
  const link = event.target.closest('a')
  linkUrl.value = link ? link.href : ''

  // 判断右键类型：链接、选中文本、或邮件操作
  if (link) {
    contextType.value = 'link'
  } else if (selectionText.value) {
    contextType.value = 'text'
  } else {
    contextType.value = 'mail'
  }

  // 确保菜单不超出屏幕
  const menuW = 180, menuH = 320
  let x = event.clientX
  let y = event.clientY
  if (x + menuW > window.innerWidth) x = window.innerWidth - menuW - 8
  if (y + menuH > window.innerHeight) y = window.innerHeight - menuH - 8
  
  contextPos.value = { x, y }
  
  showContext.value = false
  requestAnimationFrame(() => {
    showContext.value = true
  })
}

function ctxAction(action) {
  switch (action) {
    case 'copy':
      if (selectionText.value) {
        navigator.clipboard.writeText(selectionText.value)
      }
      break
    case 'copyLink':
      if (linkUrl.value) {
        navigator.clipboard.writeText(linkUrl.value)
      }
      break
    case 'openLink':
      if (linkUrl.value && window.electronAPI) {
        window.electronAPI.openExternal(linkUrl.value)
      } else if (linkUrl.value) {
        window.open(linkUrl.value, '_blank')
      }
      break
    case 'selectAll':
      if (bodyRef.value) {
        const range = document.createRange()
        range.selectNodeContents(bodyRef.value)
        const sel = window.getSelection()
        sel.removeAllRanges()
        sel.addRange(range)
      }
      break
    case 'toggleRead':
      if (props.mail.seen) {
        emit('markUnread', props.mail)
      } else {
        emit('markRead', props.mail)
      }
      break
    case 'reply':
      emit('reply', props.mail)
      break
    case 'forward':
      emit('forward', props.mail)
      break
    case 'moveTrash':
      emit('moveTrash', props.mail)
      break
    case 'delete':
      emit('delete', props.mail.uid)
      break
    case 'copyAddr':
      if (props.mail.from?.address) {
        navigator.clipboard.writeText(props.mail.from.address)
      }
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
// --- 右键菜单结束 ---

function getAvatarColor(email) {
  const colors = [
    'linear-gradient(135deg, #6c5ce7, #a29bfe)',
    'linear-gradient(135deg, #00cec9, #55efc4)',
    'linear-gradient(135deg, #fd79a8, #e84393)',
    'linear-gradient(135deg, #fdcb6e, #f39c12)',
    'linear-gradient(135deg, #74b9ff, #0984e3)',
    'linear-gradient(135deg, #55efc4, #00b894)',
    'linear-gradient(135deg, #fab1a0, #e17055)',
  ]
  let hash = 0
  for (let i = 0; i < (email || '').length; i++) {
    hash = email.charCodeAt(i) + ((hash << 5) - hash)
  }
  return colors[Math.abs(hash) % colors.length]
}

function getInitial(name) {
  if (!name) return '?'
  const trimmed = name.trim()
  if (/[\u4e00-\u9fa5]/.test(trimmed)) {
    return trimmed.charAt(trimmed.length - 1)
  }
  return trimmed.charAt(0).toUpperCase()
}

function formatFullDate(dateStr) {
  if (!dateStr) return ''
  const d = new Date(dateStr)
  return d.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function formatSize(bytes) {
  if (!bytes) return ''
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1048576).toFixed(1) + ' MB'
}

function handleBodyClick(e) {
  const link = e.target.closest('a')
  if (link && link.href) {
    e.preventDefault()
    // 检查协议，确保安全和兼容性
    const url = link.href
    if (url.startsWith('http') || url.startsWith('mailto:')) {
      if (window.electronAPI && window.electronAPI.openExternal) {
        window.electronAPI.openExternal(url)
      }
    }
  }
}

async function handleDownload(index, filename) {
  if (!window.electronAPI || !props.mail) return
  
  try {
    // 检查 mail 对象中是否有必需的元数据
    const accountId = props.mail.accountId
    const folder = props.mail.folder
    const uid = props.mail.uid
    
    if (!accountId || !folder || !uid) {
      console.warn('[MailContent] Missing download metadata:', { accountId, folder, uid })
      return
    }

    await window.electronAPI.downloadAttachment({
      accountId,
      folder,
      uid,
      index,
      filename
    })
  } catch (err) {
    console.error('Download error:', err)
  }
}

</script>

<style scoped>
.mailcontent {
  flex: 1;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
  position: relative;
}

.main-window-controls {
  position: absolute;
  top: 0;
  right: 0;
  z-index: 1000;
}

.mailcontent-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  color: var(--text-tertiary);
  font-size: 14px;
}

.empty-icon {
  opacity: 0.15;
}

.empty-text {
  color: var(--text-secondary);
  font-size: 15px;
}

.compose-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
  padding: 10px 24px;
  border-radius: var(--radius-lg);
  font-size: 14px;
}

.loading-spinner {
  width: 28px;
  height: 28px;
  border: 2px solid var(--glass-border);
  border-top-color: var(--primary-light);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* 邮件头部 */
.mail-detail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 40px 24px 12px; /* 增加顶距以避开窗口控制按钮 */
  gap: 16px;
  flex-shrink: 0;
}

.mail-detail-subject {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.4;
  flex: 1;
  min-width: 0;
  word-break: break-word;
}

.mail-detail-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.action-btn {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: rgba(255, 255, 255, 0.06);
  color: var(--text-secondary);
  border: 1px solid var(--glass-border);
}

.action-btn:hover {
  background: var(--glass-bg-hover);
  color: var(--text-primary);
  border-color: var(--glass-border-light);
}

.action-btn.danger:hover {
  background: rgba(255, 107, 107, 0.15);
  color: var(--danger);
  border-color: rgba(255, 107, 107, 0.3);
}

/* 邮件元信息 */
.mail-detail-meta {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px 24px;
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
}

.meta-avatar {
  width: 42px;
  height: 42px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-size: 16px;
  font-weight: 600;
  flex-shrink: 0;
}

.meta-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.meta-from {
  display: flex;
  align-items: baseline;
  gap: 8px;
  flex-wrap: wrap;
}

.from-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.from-addr {
  font-size: 12px;
  color: var(--text-tertiary);
}

.meta-to, .meta-cc {
  font-size: 12px;
  color: var(--text-tertiary);
}

.meta-date {
  font-size: 12px;
  color: var(--text-tertiary);
  white-space: nowrap;
  flex-shrink: 0;
}

/* 邮件正文 */
.mail-detail-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}

.mail-html-content {
  font-size: 14px;
  line-height: 1.7;
  color: var(--text-primary);
  word-break: break-word;
  background-color: transparent;
  transition: all 0.3s ease;
  color-scheme: light dark; /* 告知浏览器此容器支持深浅色模式 */
}

/* 邮件内容深浅色模式兼容性优化 */
:root:not([data-theme="light"]) .mail-html-content.dynamic-theme {
  /* 
     改进的反色逻辑：
     1. 强制设置一个白背景作为基础，确保即使是透明背景的邮件也能正常反转。
     2. 增加 brightness 提升文本亮度，降低反色后的昏暗感。
  */
  background-color: #ffffff !important;
  color: #000000 !important;
  filter: invert(1) hue-rotate(180deg) brightness(1.15) contrast(0.9);
  padding: 16px;
  border-radius: var(--radius-md);
  margin: -4px;
}

/* 
   即使关闭了动态主题，在深色模式下也要确保基础可读性。
   不使用 !important，允许邮件自身的 style 覆盖。
*/
:root:not([data-theme="light"]) .mail-html-content:not(.dynamic-theme) {
  color: var(--text-primary);
}

/* 
   媒体元素反色优化：
   1. 移除针对所有图片的全局反色（这会导致很多 Logo/图标在深色模式下消失）。
   2. 只针对可能导致“亮瞎眼”的大图或包含特定属性的图片进行反色处理。
   3. 对 iframe 和 video 保持反色以维持沉浸感。
*/
:root:not([data-theme="light"]) .mail-html-content.dynamic-theme :deep(iframe),
:root:not([data-theme="light"]) .mail-html-content.dynamic-theme :deep(video) {
  filter: invert(1) hue-rotate(180deg);
}

/* 
   让图片不再双重反转，这样黑色的 Logo 会变成白色的 Logo（在反色后的背景上），
   这正是我们在深色模式下想要的效果。
*/
.mail-html-content :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: var(--radius-sm);
  /* 针对反转后的深色背景，给大图增加一点透明度或亮度调整 */
}

:root:not([data-theme="light"]) .mail-html-content.dynamic-theme :deep(img) {
  /* 不再应用 invert，让它们跟随父级一次反转。
     如果用户觉得照片看起来怪异，我们可以再细化。
  */
  opacity: 0.9;
}

.mail-html-content :deep(a) {
  color: var(--primary-light);
  text-decoration: none;
}

:root:not([data-theme="light"]) .mail-html-content.dynamic-theme :deep(a) {
  /* 在反转后，链接颜色也需要特殊处理以保持可读性 */
  color: #0056b3; /* 这是反转前的颜色，反转后会变亮 */
}

.mail-html-content :deep(a:hover) {
  text-decoration: underline;
}

.mail-html-content :deep(table) {
  max-width: 100%;
  border-collapse: collapse;
}

.mail-html-content :deep(blockquote) {
  border-left: 3px solid var(--glass-border-light);
  padding-left: 12px;
  margin-left: 0;
  color: var(--text-secondary);
}

.mail-text-content {
  font-size: 14px;
  line-height: 1.7;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
  font-family: inherit;
}

.mail-no-content {
  color: var(--text-tertiary);
  font-style: italic;
}

/* 附件 */
.mail-detail-attachments {
  padding: 16px 24px;
  border-top: 1px solid var(--glass-border);
  flex-shrink: 0;
}

.att-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}

.att-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.att-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.att-item:hover {
  background: var(--glass-bg-hover);
  border-color: var(--glass-border-light);
}

.att-name {
  font-size: 12px;
  color: var(--text-primary);
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.att-size {
  font-size: 11px;
  color: var(--text-tertiary);
}
.pulse-text {
  animation: pulse 1.5s ease-in-out infinite;
  color: var(--text-tertiary);
  font-size: 14px;
}

@keyframes pulse {
  0% { opacity: 0.4; }
  50% { opacity: 1; }
  100% { opacity: 0.4; }
}

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
</style>
