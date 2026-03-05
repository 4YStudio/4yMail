<template>
  <div class="app-container">
    <TitleBar />
    <div class="app-body">
      <SideNav
        :activeView="currentView"
        @navigate="handleNavigate"
        @switchFolder="handleSwitchFolder"
      />
      <template v-if="currentView === 'settings'">
        <SettingsPanel
          :accountConfig="accountConfig"
          :connected="connected"
          @save="handleSaveConfig"
          @connect="handleConnect"
          @disconnect="handleDisconnect"
        />
      </template>
      <template v-else-if="currentView === 'compose'">
        <ComposeMail
          :accountConfig="accountConfig"
          :replyTo="replyToMail"
          @sent="handleMailSent"
          @back="currentView = lastMailView"
        />
      </template>
      <template v-else>
        <MailList
          :mails="mails"
          :selectedMail="selectedMailUid"
          :currentFolder="currentFolder"
          :loading="loadingList"
          :folders="folders"
          @select="handleSelectMail"
          @refresh="handleRefresh"
          @changeFolder="handleChangeFolder"
        />
        <MailContent
          :mail="currentMailDetail"
          :loading="loadingDetail"
          @reply="handleReply"
          @delete="handleDelete"
          @compose="currentView = 'compose'; replyToMail = null;"
        />
      </template>
    </div>

    <!-- 状态栏通知 -->
    <transition name="slide-up">
      <div v-if="notification" class="notification" :class="notification.type">
        <span>{{ notification.message }}</span>
        <button class="notif-close" @click="notification = null">✕</button>
      </div>
    </transition>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import TitleBar from './components/TitleBar.vue'
import SideNav from './components/SideNav.vue'
import MailList from './components/MailList.vue'
import MailContent from './components/MailContent.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import ComposeMail from './components/ComposeMail.vue'

const currentView = ref('inbox')
const lastMailView = ref('inbox')
const currentFolder = ref('INBOX')
const mails = ref([])
const folders = ref([])
const selectedMailUid = ref(null)
const currentMailDetail = ref(null)
const loadingList = ref(false)
const loadingDetail = ref(false)
const connected = ref(false)
const notification = ref(null)
const replyToMail = ref(null)

const accountConfig = reactive({
  imap: { host: '', port: 993, secure: true, user: '', pass: '' },
  smtp: { host: '', port: 465, secure: true, user: '', pass: '' },
})

// 检测是否在 Electron 环境中
const isElectron = typeof window !== 'undefined' && window.electronAPI

// 将 reactive 对象转为纯对象，避免 Electron IPC 克隆错误
function toPlain(obj) {
  return JSON.parse(JSON.stringify(obj))
}

function showNotif(message, type = 'info') {
  notification.value = { message, type }
  setTimeout(() => { notification.value = null }, 3500)
}

// 文件夹路径映射（侧边栏按钮 -> 实际文件夹名）
const folderMap = {
  'inbox': 'INBOX',
  'sent': 'Sent Messages',
  'drafts': 'Drafts',
  'trash': 'Deleted Messages',
}

// 从 localStorage 加载配置 + 自动连接
onMounted(async () => {
  const saved = localStorage.getItem('4ymail-config')
  if (saved) {
    try {
      const config = JSON.parse(saved)
      Object.assign(accountConfig, config)
    } catch {}
  }

  // 自动连接：如果有已保存的配置且有服务器地址
  if (accountConfig.imap.host && accountConfig.imap.user && accountConfig.imap.pass) {
    await handleConnect()
  }
})

function handleNavigate(view) {
  if (view !== 'compose' && view !== 'settings') {
    lastMailView.value = view
  }
  currentView.value = view
}

function handleSwitchFolder(folder) {
  // 从侧边栏切换文件夹时，先尝试智能匹配已知文件夹
  const targetFolder = findMatchingFolder(folder)
  if (targetFolder !== currentFolder.value) {
    currentFolder.value = targetFolder
    selectedMailUid.value = null
    currentMailDetail.value = null
    if (connected.value) {
      handleRefresh()
    }
  }
}

function findMatchingFolder(folderName) {
  // 在已知文件夹列表中查找匹配的文件夹
  if (!folders.value.length) return folderName
  
  // 精确匹配
  const exact = folders.value.find(f => f.path === folderName)
  if (exact) return exact.path

  // 按 specialUse 匹配
  const specialUseMap = {
    'INBOX': '\\Inbox',
    'Sent Messages': '\\Sent',
    'Drafts': '\\Drafts',
    'Deleted Messages': '\\Trash',
  }
  const specialUse = specialUseMap[folderName]
  if (specialUse) {
    const found = folders.value.find(f => f.specialUse === specialUse)
    if (found) return found.path
  }

  // 名称模糊匹配
  const nameLower = folderName.toLowerCase()
  const fuzzy = folders.value.find(f => f.name.toLowerCase().includes(nameLower) || f.path.toLowerCase().includes(nameLower))
  if (fuzzy) return fuzzy.path

  return folderName
}

function handleSaveConfig(config) {
  Object.assign(accountConfig, config)
  localStorage.setItem('4ymail-config', JSON.stringify(toPlain(accountConfig)))
  showNotif('配置已保存', 'success')
}

async function handleConnect() {
  if (!isElectron) {
    showNotif('请在桌面应用中使用邮件功能', 'warning')
    return
  }
  try {
    loadingList.value = true
    const res = await window.electronAPI.connect(toPlain(accountConfig.imap))
    if (res.success) {
      connected.value = true
      showNotif('连接成功', 'success')
      // 获取文件夹
      const fRes = await window.electronAPI.getFolders()
      if (fRes.success) {
        folders.value = fRes.data
      }
      // 加载收件箱
      await handleRefresh()
      if (currentView.value === 'settings') {
        currentView.value = 'inbox'
      }
    } else {
      showNotif('连接失败: ' + res.error, 'error')
    }
  } catch (err) {
    showNotif('连接失败: ' + err.message, 'error')
  } finally {
    loadingList.value = false
  }
}

async function handleDisconnect() {
  if (!isElectron) return
  try {
    await window.electronAPI.disconnect()
    connected.value = false
    mails.value = []
    folders.value = []
    currentMailDetail.value = null
    showNotif('已断开连接', 'info')
  } catch (err) {
    showNotif('断开失败: ' + err.message, 'error')
  }
}

async function handleRefresh() {
  if (!isElectron || !connected.value) return
  try {
    loadingList.value = true
    const res = await window.electronAPI.getMessages({
      folder: currentFolder.value,
      page: 1,
      pageSize: 50,
    })
    if (res.success) {
      mails.value = res.data.messages
    } else {
      showNotif('加载失败: ' + res.error, 'error')
    }
  } catch (err) {
    showNotif('加载失败: ' + err.message, 'error')
  } finally {
    loadingList.value = false
  }
}

async function handleChangeFolder(folder) {
  currentFolder.value = folder
  selectedMailUid.value = null
  currentMailDetail.value = null
  await handleRefresh()
}

async function handleSelectMail(uid) {
  selectedMailUid.value = uid
  if (!isElectron || !connected.value) return
  try {
    loadingDetail.value = true
    const res = await window.electronAPI.getMessage({
      folder: currentFolder.value,
      uid,
    })
    if (res.success) {
      currentMailDetail.value = res.data
      // 更新列表中的已读状态
      const idx = mails.value.findIndex(m => m.uid === uid)
      if (idx !== -1) mails.value[idx].seen = true
    } else {
      showNotif('加载邮件失败: ' + res.error, 'error')
    }
  } catch (err) {
    showNotif('加载邮件失败: ' + err.message, 'error')
  } finally {
    loadingDetail.value = false
  }
}

function handleReply(mail) {
  replyToMail.value = mail
  currentView.value = 'compose'
}

async function handleDelete(uid) {
  if (!isElectron || !connected.value) return
  try {
    const res = await window.electronAPI.deleteMail({
      folder: currentFolder.value,
      uid,
    })
    if (res.success) {
      mails.value = mails.value.filter(m => m.uid !== uid)
      if (selectedMailUid.value === uid) {
        selectedMailUid.value = null
        currentMailDetail.value = null
      }
      showNotif('邮件已删除', 'success')
    } else {
      showNotif('删除失败: ' + res.error, 'error')
    }
  } catch (err) {
    showNotif('删除失败: ' + err.message, 'error')
  }
}

function handleMailSent() {
  currentView.value = lastMailView.value
  replyToMail.value = null
  showNotif('邮件已发送', 'success')
}
</script>

<style scoped>
.app-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.app-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.notification {
  position: fixed;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  padding: 10px 24px 10px 18px;
  border-radius: var(--radius-md);
  font-size: 13px;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 12px;
  z-index: 9999;
  border: 1px solid var(--glass-border-light);
  box-shadow: 0 8px 32px rgba(0,0,0,0.3);
}

.notification.info {
  background: rgba(116, 185, 255, 0.18);
  color: var(--info);
}

.notification.success {
  background: rgba(85, 239, 196, 0.18);
  color: var(--success);
}

.notification.warning {
  background: rgba(254, 202, 87, 0.18);
  color: var(--warning);
}

.notification.error {
  background: rgba(255, 107, 107, 0.18);
  color: var(--danger);
}

.notif-close {
  background: none;
  color: inherit;
  opacity: 0.6;
  font-size: 11px;
  padding: 2px;
}

.notif-close:hover {
  opacity: 1;
}
</style>
