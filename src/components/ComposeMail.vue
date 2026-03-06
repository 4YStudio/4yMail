<template>
  <div class="compose-panel stagger-in">
    <WindowControls class="compose-window-controls" />
    <div class="compose-header">
      <div class="header-left">
        <button class="back-btn" @click="$emit('back')">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
            <path d="M19 12H5M12 19l-7-7 7-7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <h3 class="compose-title">{{ replyTo ? '回复邮件' : '写新邮件' }}</h3>
      </div>
      
      <div class="header-actions">
        <button class="btn-glass" @click="handleSaveDraft" :disabled="sending || savingDraft">
          <div v-if="savingDraft" class="btn-spinner mini"></div>
          {{ savingDraft ? '保存中' : '存草稿' }}
        </button>
        <button class="btn-glass attachment-btn" @click="handlePickAttachments" :disabled="sending">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
            <path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span v-if="attachments.length > 0" class="attach-count">{{ attachments.length }}</span>
        </button>
        <button class="btn-glass clear-btn" @click="clearForm" title="清空" :disabled="sending">
          清空
        </button>
        <button class="btn-primary send-btn" @click="sendMail" :disabled="sending || !mailData.to">
          <div v-if="sending" class="btn-spinner"></div>
          {{ sending ? '发送中' : '发送' }}
        </button>
      </div>
    </div>

    <div class="compose-body">
      <div class="compose-fields">
        <div class="field-item">
          <label>收件人</label>
          <input v-model="mailData.to" type="text" placeholder="邮件地址，多个用逗号分隔" />
        </div>
        <div class="field-item">
          <label>抄送</label>
          <input v-model="mailData.cc" type="text" placeholder="可选" />
        </div>
        <div class="field-item">
          <label>主题</label>
          <input v-model="mailData.subject" type="text" placeholder="输入主题" />
        </div>
      </div>

      <!-- 附件列表 -->
      <div v-if="attachments.length > 0" class="attachments-list">
        <div v-for="(file, index) in attachments" :key="index" class="attachment-item">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
            <path d="M13 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V9z" stroke="currentColor" stroke-width="1.5"/>
            <path d="M13 2v7h7" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <span class="file-name" :title="file.path">{{ file.name }}</span>
          <button class="remove-attach" @click="removeAttachment(index)">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
      </div>

      <!-- 编辑器工具栏 -->
      <div class="editor-toolbar">
        <button @click="exec('bold')" title="加粗" class="tool-btn"><B>B</B></button>
        <button @click="exec('italic')" title="斜体" class="tool-btn"><I>I</I></button>
        <button @click="exec('underline')" title="下划线" class="tool-btn"><U>U</U></button>
        <div class="tool-divider"></div>
        <button @click="exec('insertOrderedList')" title="有序列表" class="tool-btn">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M10 6h11M10 12h11M10 18h11M4 6h1v4M4 10h2M4 18h2M4 15a2 2 0 112 2M4 15v2"/>
          </svg>
        </button>
        <button @click="exec('insertUnorderedList')" title="无序列表" class="tool-btn">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M10 6h11M10 12h11M10 18h11M4 6h.01M4 12h.01M4 18h.01"/>
          </svg>
        </button>
        <div class="tool-divider"></div>
        <button @click="exec('justifyLeft')" title="左对齐" class="tool-btn">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M17 10H3M21 6H3M21 14H3M17 18H3"/></svg>
        </button>
        <button @click="exec('justifyCenter')" title="居中" class="tool-btn">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M18 10H6M21 6H3M21 14H3M18 18H6"/></svg>
        </button>
      </div>

      <div class="compose-editor">
        <div
          contenteditable="true"
          class="rich-editor"
          ref="editorRef"
          @input="updateBody"
          placeholder="在此输入邮件内容..."
        ></div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, onUnmounted, nextTick } from 'vue'
import WindowControls from './WindowControls.vue'

const props = defineProps({
  accountConfig: Object,
  replyTo: Object,
})

const emit = defineEmits(['sent', 'back', 'notif'])

const isElectron = typeof window !== 'undefined' && window.electronAPI
const sending = ref(false)
const savingDraft = ref(false)
const editorRef = ref(null)
const attachments = ref([])

const mailData = reactive({
  to: '',
  cc: '',
  subject: '',
  body: '', // 这里存储 HTML 内容
})

onMounted(() => {
  if (props.replyTo) {
    mailData.to = props.replyTo.from?.address || ''
    mailData.subject = 'Re: ' + (props.replyTo.subject || '')
    const originalContent = props.replyTo.html || props.replyTo.text || ''
    mailData.body = `<br><br><div class="gmail_quote">--- 原始邮件 ---<br>${originalContent}</div>`
    
    if (editorRef.value) {
      editorRef.value.innerHTML = mailData.body
    }
  }
  
  // 聚焦编辑器
  nextTick(() => {
    if (editorRef.value) {
      editorRef.value.focus()
    }
  })
})

function updateBody() {
  if (editorRef.value) {
    mailData.body = editorRef.value.innerHTML
  }
}

function exec(command, value = null) {
  document.execCommand(command, false, value)
  updateBody()
  editorRef.value.focus()
}

async function handlePickAttachments() {
  if (!isElectron) return
  const result = await window.electronAPI.openFileDialog()
  if (!result.canceled && result.filePaths.length > 0) {
    result.filePaths.forEach(filePath => {
      const fileName = filePath.split(/[/\\]/).pop()
      // 避免重复添加
      if (!attachments.value.find(a => a.path === filePath)) {
        attachments.value.push({
          path: filePath,
          name: fileName
        })
      }
    })
  }
}

function removeAttachment(index) {
  attachments.value.splice(index, 1)
}

function clearForm() {
  if (!confirm('确定要清空当前内容吗？')) return
  mailData.to = ''
  mailData.cc = ''
  mailData.subject = ''
  mailData.body = ''
  attachments.value = []
  if (editorRef.value) {
    editorRef.value.innerHTML = ''
    editorRef.value.focus()
  }
}

async function handleSaveDraft() {
  if (!isElectron || !props.accountConfig) return
  
  if (typeof window.electronAPI.saveDraft !== 'function') {
    console.error('API Error: saveDraft is not defined in window.electronAPI');
    emit('notif', { message: '保存功能不可用，请重启软件以更新', type: 'error' })
    return
  }
  
  savingDraft.value = true
  try {
    // 简单生成原始邮件格式（实际应使用专门的库生成 RFC822）
    const draftContent = `Subject: ${mailData.subject}\nTo: ${mailData.to}\nContent-Type: text/html; charset=utf-8\n\n${mailData.body}`;
    
    const res = await window.electronAPI.saveDraft({
      accountId: props.accountConfig.id,
      content: draftContent
    })
    
    if (res.success) {
      emit('notif', { message: '已存入草稿箱', type: 'success' })
    } else {
      emit('notif', { message: '保存失败: ' + res.error, type: 'error' })
    }
  } catch (err) {
    emit('notif', { message: '保存失败: ' + err.message, type: 'error' })
  } finally {
    savingDraft.value = false
  }
}

// 辅助函数：将响应式对象转为纯对象
function toRawObject(obj) {
  return JSON.parse(JSON.stringify(obj))
}

async function sendMail() {
  if (!mailData.to.trim()) return
  if (!isElectron) return
  
  if (typeof window.electronAPI.sendMail !== 'function') {
    emit('notif', { message: '系统 API 未就绪，请尝试重启软件', type: 'error' })
    return
  }
  
  // 补全 SMTP 凭据（某些旧账号可能没存 SMTP user/pass）
  if (props.accountConfig && props.accountConfig.smtp && !props.accountConfig.smtp.user) {
    props.accountConfig.smtp.user = props.accountConfig.imap.user
    props.accountConfig.smtp.pass = props.accountConfig.imap.pass
  }

  if (!props.accountConfig || !props.accountConfig.smtp || !props.accountConfig.smtp.user) {
    emit('notif', { message: '发件配置缺失，请在设置中重新保存账号', type: 'error' })
    return
  }

  sending.value = true
  try {
    // 克隆配置以防引用丢失
    const smtp = toRawObject(props.accountConfig.smtp)
    const options = {
      from: smtp.user,
      to: mailData.to,
      cc: mailData.cc || undefined,
      subject: mailData.subject,
      html: mailData.body,
      attachments: attachments.value.map(a => ({
        filename: a.name,
        path: a.path
      }))
    }

    const res = await window.electronAPI.sendMail({
      smtpConfig: smtp,
      mailOptions: options
    })
    
    if (res.success) {
      emit('sent')
    } else {
      emit('notif', { message: '发送失败: ' + res.error, type: 'error' })
    }
  } catch (err) {
    emit('notif', { message: '发送失败: ' + err.message, type: 'error' })
  } finally {
    sending.value = false
  }
}
</script>

<style scoped>
.compose-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg-main);
  position: relative;
}

.compose-window-controls {
  position: absolute;
  top: 0;
  right: 0;
  z-index: 1000;
}

.compose-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 40px 24px 12px;
  border-bottom: 1px solid var(--glass-border);
  background: var(--glass-bg);
  backdrop-filter: blur(10px);
  flex-shrink: 0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.header-actions .divider {
  width: 1px;
  height: 20px;
  background: var(--glass-border);
  margin: 0 4px;
}

.back-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  transition: all 0.2s;
  background: transparent;
  border: none;
  color: var(--text-primary);
  cursor: pointer;
}

.back-btn:hover {
  background: var(--glass-bg-hover);
}

.compose-title {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.btn-glass {
  padding: 6px 12px;
  border-radius: 8px;
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 6px;
  transition: all 0.2s;
  background: var(--glass-bg);
  border: 1px solid var(--glass-border);
  color: var(--text-primary);
}

.btn-glass:hover:not(:disabled) {
  background: var(--glass-bg-hover);
}

.attachment-btn {
  color: var(--primary);
  border: 1px solid rgba(var(--primary-rgb), 0.2);
}

.attach-count {
  font-size: 11px;
  background: var(--primary);
  color: white;
  min-width: 16px;
  height: 16px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 4px;
}

.send-btn {
  padding: 6px 18px;
  border-radius: 8px;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 8px;
}

.compose-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  padding: 0;
}

.compose-fields {
  padding: 0 20px;
  border-bottom: 1px solid var(--glass-border);
}

.field-item {
  display: flex;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid var(--glass-border-light);
}

.field-item:last-child {
  border-bottom: none;
}

.field-item label {
  width: 60px;
  font-size: 13px;
  color: var(--text-tertiary);
}

.field-item input {
  flex: 1;
  background: transparent;
  border: none;
  font-size: 14px;
  color: var(--text-primary);
  outline: none;
}

/* 附件样式 */
.attachments-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 12px 20px;
  background: var(--glass-bg-subtle);
  border-bottom: 1px solid var(--glass-border);
}

.attachment-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  background: var(--bg-card);
  border: 1px solid var(--glass-border);
  border-radius: 6px;
  max-width: 200px;
}

.file-name {
  flex: 1;
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--text-secondary);
}

.remove-attach {
  background: transparent;
  border: none;
  color: var(--text-tertiary);
  padding: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: all 0.2s;
}

.remove-attach:hover {
  background: rgba(255, 77, 77, 0.1);
  color: #ff4d4d;
}

/* 工具栏样式 */
.editor-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 20px;
  border-bottom: 1px solid var(--glass-border);
  background: var(--glass-bg-subtle);
  position: sticky;
  top: 0;
  z-index: 10;
}

:root[data-theme="dark"] .editor-toolbar {
  background: rgba(255, 255, 255, 0.03);
}

.tool-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  color: var(--text-secondary);
  transition: all 0.2s;
  background: transparent;
  border: none;
  cursor: pointer;
}

.tool-btn:hover {
  background: var(--glass-bg-hover);
  color: var(--primary);
}

.tool-divider {
  width: 1px;
  height: 16px;
  background: var(--glass-border);
  margin: 0 6px;
}

/* 编辑器容器 */
.compose-editor {
  flex: 1;
  padding: 20px;
  min-height: 400px;
}

.rich-editor {
  width: 100%;
  height: 100%;
  min-height: 350px;
  outline: none;
  font-size: 15px;
  line-height: 1.6;
  color: var(--text-primary);
  word-break: break-all;
}

.rich-editor:empty::before {
  content: attr(placeholder);
  color: var(--text-tertiary);
  cursor: text;
}

.btn-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255,255,255,0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  display: inline-block;
}

.btn-spinner.mini {
  width: 12px;
  height: 12px;
  border-width: 1.5px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
