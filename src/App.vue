<template>
  <div class="app-container">
    <div class="window-drag-region"></div>
    <div class="app-body">
      <SideNav
        :activeView="currentView"
        :accounts="accounts"
        :currentAccountId="currentAccountId"
        @navigate="handleNavigate"
        @switchFolder="handleSwitchFolder"
        @switchAccount="id => currentAccountId = id"
      />
      <Transition :name="transitionName" mode="out-in">
        <template v-if="currentView === 'settings'">
          <SettingsPanel
            :key="'settings'"
            :accounts="accounts"
            :pollingInterval="pollingInterval"
            :zoomLevel="zoomLevel"
            :autoStart="autoStart"
            :connectingIds="Array.from(connectingIds)"
            @updateInterval="val => pollingInterval = val"
            @updateZoom="val => zoomLevel = val"
            @updateAutoStart="val => autoStart = val"
            @reconnect="handleConnect"
            @addAccount="onAddAccount"
            @updateAccount="onUpdateAccount"
            @deleteAccount="onDeleteAccount"
          />
        </template>
        <template v-else-if="currentView === 'compose'">
          <ComposeMail
            :key="'compose'"
            :accountConfig="currentAccount || accounts[0]"
            :replyTo="replyToMail"
            @sent="handleMailSent"
            @back="currentView = lastMailView"
            @notif="n => showNotif(n.message, n.type)"
          />
        </template>
        <template v-else>
          <div class="mail-view-layout" :key="'mails'">
            <MailList
              :mails="mails"
              :selectedMail="selectedMailUid"
              :currentFolder="currentFolder"
              :loading="loadingList"
              :folders="folders"
              @select="handleSelectMail"
              @refresh="handleRefresh"
              @changeFolder="handleChangeFolder"
              @markRead="handleMarkRead"
              @markUnread="handleMarkUnread"
              @reply="handleCtxReply"
              @delete="handleCtxDelete"
              @moveTrash="handleMoveTrash"
              @copyAddr="handleCopyAddr"
            />
            <Transition name="content-slide" mode="out-in">
              <MailContent
                :key="selectedMailUid"
                :mail="currentMailDetail"
                :loading="loadingDetail"
                @reply="handleReply"
                @delete="handleDelete"
                @compose="currentView = 'compose'; replyToMail = null;"
              />
            </Transition>
          </div>
        </template>
      </Transition>
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
import { ref, reactive, onMounted, computed, watch } from 'vue'
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
const notification = ref(null)
const replyToMail = ref(null)

// 多账号支持
const accounts = ref([]) // { id, imap, smtp, connected, folders: [] }
const currentAccountId = ref('unified') // 'unified' 或具体的 email 地址
const pollingInterval = ref(5)
const zoomLevel = ref(100) // 100%
const autoStart = ref(false)
const connectingIds = ref(new Set())

// 当前选中账号的对象
const currentAccount = computed(() => {
  if (currentAccountId.value === 'unified') return null
  return accounts.value.find(a => a.id === currentAccountId.value)
})

const transitionName = computed(() => {
  if (currentView.value === 'compose') return 'route-compose'
  if (currentView.value === 'settings') return 'route-settings'
  return 'route-slide'
})

const connected = computed(() => {
  if (currentAccountId.value === 'unified') {
    return accounts.value.some(a => a.connected)
  }
  return currentAccount.value?.connected || false
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
  const savedAccounts = localStorage.getItem('4ymail-accounts')
  if (savedAccounts) {
    try {
      accounts.value = JSON.parse(savedAccounts).map(a => ({ ...a, connected: false, folders: [] }))
    } catch {}
  }

  const savedSettings = localStorage.getItem('4ymail-settings')
  if (savedSettings) {
    try {
      const settings = JSON.parse(savedSettings)
      pollingInterval.value = settings.pollingInterval || 5
      zoomLevel.value = settings.zoomLevel || 100
      if (isElectron) {
        window.electronAPI.updatePollingInterval(pollingInterval.value)
        window.electronAPI.setZoomFactor(zoomLevel.value / 100)
      }
    } catch {}
  }

  // 初始化自启动状态
  if (isElectron) {
    const asRes = await window.electronAPI.getAutostart()
    if (asRes.success) autoStart.value = asRes.enabled
  }

  // 监听来自主进程的连接异常
  if (isElectron) {
    window.electronAPI.onConnectionError(({ accountId, error }) => {
      showNotif(`账号 ${accountId} 连接断开: ${error}`, 'error')
      const acc = accounts.value.find(a => a.id === accountId)
      if (acc) {
        acc.connected = false
        if (currentAccountId.value === accountId) {
          mails.value = []
          folders.value = []
        }
      }
    })
    window.electronAPI.onNewMailArrived(({ accountId }) => {
      // 如果当前在收件箱查看，且涉及该账号或统一视图，则刷新列表
      if (currentFolder.value === 'INBOX' && (currentAccountId.value === 'unified' || currentAccountId.value === accountId)) {
        handleRefresh()
      }
    })
  }

  // 自动连接所有已保存的账号
  for (const acc of accounts.value) {
    if (acc.imap.host && acc.imap.user) {
      handleConnect(acc.id)
    }
  }
})

// 监听账号变化持久化
watch(accounts, (newVal) => {
  const toSave = newVal.map(a => {
    const { connected, folders, ...rest } = a
    return rest
  })
  localStorage.setItem('4ymail-accounts', JSON.stringify(toSave))
}, { deep: true })

watch(pollingInterval, (newVal) => {
  saveSettings()
  if (isElectron) window.electronAPI.updatePollingInterval(newVal)
})

watch(zoomLevel, (newVal) => {
  saveSettings()
  if (isElectron) window.electronAPI.setZoomFactor(newVal / 100)
})

watch(autoStart, (newVal) => {
  if (isElectron) window.electronAPI.setAutostart(newVal)
})

function saveSettings() {
  localStorage.setItem('4ymail-settings', JSON.stringify({
    pollingInterval: pollingInterval.value,
    zoomLevel: zoomLevel.value
  }))
}

// 切换账号时自动加载文件夹和邮件
watch(currentAccountId, async (newId) => {
  selectedMailUid.value = null
  currentMailDetail.value = null

  if (newId === 'unified') {
    folders.value = []
    currentFolder.value = 'INBOX'
  } else {
    const acc = accounts.value.find(a => a.id === newId)
    if (acc && acc.connected) {
      folders.value = acc.folders || []
    } else {
      folders.value = []
    }
    currentFolder.value = 'INBOX'
  }
  if (connected.value) {
    mails.value = [] // 切换账号时清空
    await handleRefresh()
  }
})

function handleNavigate(view) {
  if (view !== 'compose' && view !== 'settings') {
    lastMailView.value = view
  }
  currentView.value = view
}

function handleSwitchFolder(folder) {
  // 从侧边栏切换文件夹
  const targetFolder = findMatchingFolder(folder)
  currentFolder.value = targetFolder
  selectedMailUid.value = null
  currentMailDetail.value = null
  mails.value = [] // 切换文件夹时清空
  if (connected.value) {
    handleRefresh()
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

// 为指定账号解析实际的 IMAP 文件夹路径
function resolveFolder(folderName, accountFolders) {
  if (!accountFolders || !accountFolders.length) return folderName

  // 精确匹配
  const exact = accountFolders.find(f => f.path === folderName)
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
    const found = accountFolders.find(f => f.specialUse === specialUse)
    if (found) return found.path
  }

  // 名称模糊匹配
  const nameLower = folderName.toLowerCase()
  const fuzzy = accountFolders.find(f =>
    f.name.toLowerCase().includes(nameLower) ||
    f.path.toLowerCase().includes(nameLower)
  )
  if (fuzzy) return fuzzy.path

  return folderName
}

function onAddAccount(acc) {
  const existing = accounts.value.find(a => a.id === acc.id)
  if (existing) {
    showNotif('该账号已存在', 'warning')
    return
  }
  accounts.value.push({ ...acc, connected: false, folders: [] })
  handleConnect(acc.id)
}

function onUpdateAccount(oldId, acc) {
  const idx = accounts.value.findIndex(a => a.id === oldId)
  if (idx !== -1) {
    accounts.value[idx] = { ...accounts.value[idx], ...acc }
    handleConnect(acc.id)
    if (oldId !== acc.id && currentAccountId.value === oldId) {
      currentAccountId.value = acc.id
    }
  }
}

async function onDeleteAccount(id) {
  await handleDisconnect(id)
  accounts.value = accounts.value.filter(a => a.id !== id)
  if (currentAccountId.value === id) {
    currentAccountId.value = 'unified'
  }
}


async function handleConnect(accountId) {
  if (!isElectron) {
    showNotif('请在桌面应用中使用邮件功能', 'warning')
    return
  }
  const acc = accounts.value.find(a => a.id === accountId)
  if (!acc) return

  if (connectingIds.value.has(accountId)) return // 已经在连接中

  try {
    connectingIds.value.add(accountId)
    loadingList.value = true
    const res = await window.electronAPI.connect({
      imap: toPlain(acc.imap),
      smtp: toPlain(acc.smtp)
    })
    if (res.success) {
      acc.connected = true
      // 获取文件夹
      const fRes = await window.electronAPI.getFolders(acc.id)
      if (fRes.success) {
        acc.folders = fRes.data
        if (currentAccountId.value === acc.id) {
          folders.value = acc.folders
        }
      }
      
      // 如果当前正在查看该账号，刷新
      if (currentAccountId.value === acc.id || currentAccountId.value === 'unified') {
        await handleRefresh()
      }
    } else {
      showNotif(`账号 ${accountId} 连接失败: ${res.error}`, 'error')
    }
  } catch (err) {
    showNotif(`连接失败: ${err.message}`, 'error')
  } finally {
    connectingIds.value.delete(accountId)
    loadingList.value = false
  }
}

async function handleDisconnect(accountId) {
  if (!isElectron) return
  const acc = accounts.value.find(a => a.id === accountId)
  if (!acc) return

  try {
    await window.electronAPI.disconnect(acc.id)
    acc.connected = false
    if (currentAccountId.value === acc.id) {
      mails.value = []
      folders.value = []
      currentMailDetail.value = null
    }
    showNotif(`账号 ${accountId} 已断开`, 'info')
  } catch (err) {
    showNotif('断开失败: ' + err.message, 'error')
  }
}

async function handleRefresh() {
  if (!isElectron) return

  // 先尝试重连断开的账号
  const disconnected = accounts.value.filter(a => !a.connected && a.imap.host && a.imap.user)
  if (disconnected.length > 0) {
    for (const acc of disconnected) {
      try {
        const res = await window.electronAPI.connect({
          imap: toPlain(acc.imap),
          smtp: toPlain(acc.smtp)
        })
        if (res.success) {
          acc.connected = true
          const fRes = await window.electronAPI.getFolders(acc.id)
          if (fRes.success) {
            acc.folders = fRes.data
            if (currentAccountId.value === acc.id) {
              folders.value = acc.folders
            }
          }
          showNotif(`账号 ${acc.id} 已重新连接`, 'success')
        }
      } catch {}
    }
  }

  if (!connected.value) {
    mails.value = []
    loadingList.value = false
    return
  }

  loadingList.value = true
  try {
    if (currentAccountId.value === 'unified') {
      const allMails = []
      for (const acc of accounts.value) {
        if (!acc.connected) continue
        // 为每个账号独立解析实际的文件夹路径
        const realFolder = resolveFolder(currentFolder.value, acc.folders || [])
        const res = await window.electronAPI.getMessages({
          accountId: acc.id,
          folder: realFolder,
          page: 1,
          pageSize: 30,
        })
        if (res.success) {
          allMails.push(...res.data.messages.map(m => ({ ...m, accountId: acc.id })))
        }
      }
      allMails.sort((a, b) => new Date(b.date) - new Date(a.date))
      mails.value = allMails
    } else {
      const res = await window.electronAPI.getMessages({
        accountId: currentAccountId.value,
        folder: currentFolder.value,
        page: 1,
        pageSize: 50,
      })
      if (res.success) {
        mails.value = res.data.messages.map(m => ({ ...m, accountId: currentAccountId.value }))
      }
    }
  } catch (err) {
    showNotif('刷新失败: ' + err.message, 'error')
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

async function handleSelectMail(mail) {
  // 现在接收的是 mail 对象
  const uid = mail.uid
  const accountId = mail.accountId || currentAccountId.value
  
  selectedMailUid.value = uid
  if (!isElectron) return
  
  try {
    loadingDetail.value = true
    
    // 关键修正：确保获取邮件详情时使用的文件夹路径与邮件所在账号匹配
    const acc = accounts.value.find(a => a.id === accountId)
    const realFolder = resolveFolder(currentFolder.value, acc?.folders || [])

    const res = await window.electronAPI.getMessage({
      accountId,
      folder: realFolder,
      uid,
    })
    if (res.success) {
      currentMailDetail.value = { ...res.data, accountId }
      // 更新列表中的已读状态
      const idx = mails.value.findIndex(m => m.uid === uid && (m.accountId === accountId))
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

// === 右键菜单处理函数 ===

async function handleMarkRead(mail) {
  if (!isElectron) return
  const accountId = mail.accountId || currentAccountId.value
  const folder = currentAccountId.value === 'unified'
    ? resolveFolder(currentFolder.value, accounts.value.find(a => a.id === accountId)?.folders || [])
    : currentFolder.value
  try {
    const res = await window.electronAPI.markSeen({ accountId, folder, uid: mail.uid })
    if (res.success) {
      const idx = mails.value.findIndex(m => m.uid === mail.uid && m.accountId === accountId)
      if (idx !== -1) mails.value[idx].seen = true
    }
  } catch (err) {
    showNotif('操作失败: ' + err.message, 'error')
  }
}

async function handleMarkUnread(mail) {
  if (!isElectron) return
  const accountId = mail.accountId || currentAccountId.value
  const folder = currentAccountId.value === 'unified'
    ? resolveFolder(currentFolder.value, accounts.value.find(a => a.id === accountId)?.folders || [])
    : currentFolder.value
  try {
    const res = await window.electronAPI.markUnseen({ accountId, folder, uid: mail.uid })
    if (res.success) {
      const idx = mails.value.findIndex(m => m.uid === mail.uid && m.accountId === accountId)
      if (idx !== -1) mails.value[idx].seen = false
    }
  } catch (err) {
    showNotif('操作失败: ' + err.message, 'error')
  }
}

async function handleCtxReply(mail) {
  // 从右键菜单回复：先加载邮件详情，然后进入回复
  const accountId = mail.accountId || currentAccountId.value
  const folder = currentAccountId.value === 'unified'
    ? resolveFolder(currentFolder.value, accounts.value.find(a => a.id === accountId)?.folders || [])
    : currentFolder.value
  try {
    const res = await window.electronAPI.getMessage({ accountId, folder, uid: mail.uid })
    if (res.success) {
      replyToMail.value = { ...res.data, accountId }
      currentView.value = 'compose'
    }
  } catch (err) {
    showNotif('加载邮件失败: ' + err.message, 'error')
  }
}

async function handleCtxDelete(mail) {
  if (!isElectron) return
  const accountId = mail.accountId || currentAccountId.value
  if (accountId === 'unified') return
  const folder = currentAccountId.value === 'unified'
    ? resolveFolder(currentFolder.value, accounts.value.find(a => a.id === accountId)?.folders || [])
    : currentFolder.value
  try {
    const res = await window.electronAPI.deleteMail({ accountId, folder, uid: mail.uid })
    if (res.success) {
      mails.value = mails.value.filter(m => !(m.uid === mail.uid && m.accountId === accountId))
      if (selectedMailUid.value === mail.uid) {
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

async function handleMoveTrash(mail) {
  if (!isElectron) return
  const accountId = mail.accountId || currentAccountId.value
  if (accountId === 'unified') return
  const acc = accounts.value.find(a => a.id === accountId)
  const folder = currentAccountId.value === 'unified'
    ? resolveFolder(currentFolder.value, acc?.folders || [])
    : currentFolder.value
  // 查找垃圾箱的实际路径
  const trashFolder = resolveFolder('Deleted Messages', acc?.folders || [])
  try {
    const res = await window.electronAPI.moveMail({ accountId, folder, uid: mail.uid, destination: trashFolder })
    if (res.success) {
      mails.value = mails.value.filter(m => !(m.uid === mail.uid && m.accountId === accountId))
      if (selectedMailUid.value === mail.uid) {
        selectedMailUid.value = null
        currentMailDetail.value = null
      }
      showNotif('已移至垃圾箱', 'success')
    } else {
      showNotif('移动失败: ' + res.error, 'error')
    }
  } catch (err) {
    showNotif('移动失败: ' + err.message, 'error')
  }
}

function handleCopyAddr(mail) {
  const addr = mail.from?.address || ''
  if (addr) {
    navigator.clipboard.writeText(addr).then(() => {
      showNotif('已复制: ' + addr, 'success')
    }).catch(() => {
      showNotif('复制失败', 'error')
    })
  }
}


async function handleDelete(uid) {
  if (!isElectron) return
  const accountId = currentMailDetail.value?.accountId || currentAccountId.value
  if (!accountId || accountId === 'unified') return
  
  try {
    const acc = accounts.value.find(a => a.id === accountId)
    const realFolder = resolveFolder(currentFolder.value, acc?.folders || [])

    const res = await window.electronAPI.deleteMail({
      accountId,
      folder: realFolder,
      uid,
    })
    if (res.success) {
      mails.value = mails.value.filter(m => !(m.uid === uid && m.accountId === accountId))
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
  position: relative;
}

.window-drag-region {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 32px;
  -webkit-app-region: drag;
  pointer-events: none;
  z-index: 999;
}

.app-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.mail-view-layout {
  flex: 1;
  display: flex;
  overflow: hidden;
  height: 100%;
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
