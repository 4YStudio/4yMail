<template>
  <div class="mailcontent">
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

      <div class="mail-detail-body selectable" ref="bodyRef">
        <div v-if="mail.html" v-html="sanitizeHtml(mail.html)" class="mail-html-content"></div>
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
          <div v-for="(att, i) in mail.attachments" :key="i" class="att-item">
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
  </div>
</template>

<script setup>
const props = defineProps({
  mail: Object,
  loading: Boolean,
})

defineEmits(['reply', 'delete', 'compose'])

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

function sanitizeHtml(html) {
  // 基础清理，移除 script 标签
  return html
    .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')
    .replace(/on\w+="[^"]*"/gi, '')
    .replace(/on\w+='[^']*'/gi, '')
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
  padding: 20px 24px 12px;
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
}

.mail-html-content :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: var(--radius-sm);
}

.mail-html-content :deep(a) {
  color: var(--primary-light);
  text-decoration: none;
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
</style>
