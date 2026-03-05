<template>
  <div class="compose-panel">
    <div class="compose-header">
      <button class="back-btn" @click="$emit('back')">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
          <path d="M19 12H5M12 19l-7-7 7-7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      <h3 class="compose-title">{{ replyTo ? '回复邮件' : '写新邮件' }}</h3>
      <button class="btn-glass clear-btn" @click="clearForm" title="清空">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
          <path d="M3 6h18M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        清空
      </button>
      <button class="btn-primary send-btn" @click="sendMail" :disabled="sending">
        <svg v-if="!sending" width="16" height="16" viewBox="0 0 24 24" fill="none">
          <path d="M22 2L11 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <path d="M22 2L15 22l-4-9-9-4L22 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
        </svg>
        <div v-else class="btn-spinner"></div>
        {{ sending ? '发送中...' : '发送' }}
      </button>
    </div>

    <div class="compose-body">
      <div class="compose-field">
        <label>收件人</label>
        <input v-model="mailData.to" type="text" placeholder="email@example.com, ..." />
      </div>
      <div class="compose-field">
        <label>抄送</label>
        <input v-model="mailData.cc" type="text" placeholder="可选" />
      </div>
      <div class="compose-field">
        <label>主题</label>
        <input v-model="mailData.subject" type="text" placeholder="邮件主题" />
      </div>
      <div class="compose-editor">
        <textarea
          v-model="mailData.body"
          placeholder="输入邮件内容..."
          ref="editorRef"
        ></textarea>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'

const props = defineProps({
  accountConfig: Object,
  replyTo: Object,
})

const emit = defineEmits(['sent', 'back'])

const sending = ref(false)
const editorRef = ref(null)

const isElectron = typeof window !== 'undefined' && window.electronAPI

const mailData = reactive({
  to: '',
  cc: '',
  subject: '',
  body: '',
})

onMounted(() => {
  if (props.replyTo) {
    mailData.to = props.replyTo.from?.address || ''
    mailData.subject = 'Re: ' + (props.replyTo.subject || '')
    mailData.body = '\n\n--- 原始邮件 ---\n' + (props.replyTo.text || '')
  }
  if (editorRef.value) {
    editorRef.value.focus()
  }
})

function clearForm() {
  mailData.to = ''
  mailData.cc = ''
  mailData.subject = ''
  mailData.body = ''
  if (editorRef.value) {
    editorRef.value.focus()
  }
}

async function sendMail() {
  if (!mailData.to.trim()) return
  if (!isElectron) return

  sending.value = true
  try {
    const res = await window.electronAPI.sendMail(JSON.parse(JSON.stringify({
      smtpConfig: props.accountConfig.smtp,
      mailOptions: {
        from: props.accountConfig.smtp.user,
        to: mailData.to,
        cc: mailData.cc || undefined,
        subject: mailData.subject,
        text: mailData.body,
      },
    })))
    if (res.success) {
      emit('sent')
    } else {
      alert('发送失败: ' + res.error)
    }
  } catch (err) {
    alert('发送失败: ' + err.message)
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
}

.compose-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 20px;
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
}

.back-btn {
  width: 34px;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
}

.back-btn:hover {
  background: var(--glass-bg-hover);
  color: var(--text-primary);
}

.compose-title {
  flex: 1;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}

.clear-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 14px;
  border-radius: var(--radius-lg);
  font-size: 12px;
}

.send-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 20px;
  border-radius: var(--radius-lg);
}

.send-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255,255,255,0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.compose-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 16px 20px;
  gap: 2px;
  overflow: hidden;
}

.compose-field {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid var(--glass-border);
}

.compose-field label {
  width: 50px;
  font-size: 13px;
  color: var(--text-tertiary);
  flex-shrink: 0;
}

.compose-field input {
  flex: 1;
  border: none;
  background: transparent;
  padding: 6px 0;
  font-size: 14px;
}

.compose-field input:focus {
  border: none;
}

.compose-editor {
  flex: 1;
  margin-top: 12px;
  overflow: hidden;
}

.compose-editor textarea {
  width: 100%;
  height: 100%;
  resize: none;
  background: transparent;
  border: none;
  font-size: 14px;
  line-height: 1.7;
  color: var(--text-primary);
  padding: 0;
}

.compose-editor textarea:focus {
  border: none;
}
</style>
