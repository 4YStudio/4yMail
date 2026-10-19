<template>
  <div class="app-container" :style="isTauri ? rootStyle : {}">
    <div class="window-drag-region" data-tauri-drag-region></div>
    <WindowControls class="app-window-controls" />
    <div class="app-body">
      <SideNav
        :activeView="currentView"
        :accounts="accounts"
        :unreadTotal="accounts.filter(a => a.enabled !== false).reduce((sum, a) => sum + (a.unreadCount || 0), 0)"
        :currentAccountId="currentAccountId"
        @navigate="handleNavigate"
        @switchFolder="handleSwitchFolder"
        @switchAccount="id => currentAccountId = id"
      />
      <div class="app-view-host">
        <Transition :name="transitionName" mode="out-in">
          <SettingsPanel
            v-if="currentView === 'settings'"
            key="settings"
            :accounts="accounts"
            :pollingInterval="pollingInterval"
            :zoomLevel="zoomLevel"
            :autoStart="autoStart"
            :dynamicMailTheme="dynamicMailTheme"
            :connectingIds="Array.from(connectingIds)"
            @updateInterval="val => pollingInterval = val"
            @updateZoom="val => zoomLevel = val"
            @updateAutoStart="val => autoStart = val"
            @updateDynamicMailTheme="val => dynamicMailTheme = val"
            @reconnect="handleConnect"
            @addAccount="onAddAccount"
            @updateAccount="onUpdateAccount"
            @deleteAccount="onDeleteAccount"
            @toggleAccount="handleToggleAccount"
          />
          <ComposeMail
            v-else-if="currentView === 'compose'"
            key="compose"
            :accountConfig="currentAccount || accounts[0]"
            :replyTo="replyToMail"
            @sent="handleMailSent"
            @back="currentView = lastMailView"
            @notif="n => showNotif(n.message, n.type)"
          />
          <div v-else class="mail-view-layout" key="mails">
            <MailList
              :mails="mails"
              :selectedMail="selectedMailUid"
              :currentFolder="currentFolder"
              :loading="loadingList"
              :folders="folders"
              :page="mailPage"
              :total="mailTotal"
              :query="mailQuery"
              :unreadOnly="unreadOnly"
              :flaggedOnly="flaggedOnly"
              @select="handleSelectMail"
              @refresh="handleRefresh"
              @changeFolder="handleChangeFolder"
              @markRead="handleMarkRead"
              @markUnread="handleMarkUnread"
              @toggleStar="handleToggleStar"
              @reply="handleCtxReply"
              @delete="handleCtxDelete"
              @moveTrash="handleMoveTrash"
              @copyAddr="handleCopyAddr"
              @page-change="handlePageChange"
              @search-change="handleSearchChange"
              @unread-change="handleUnreadFilterChange"
              @flagged-change="handleFlaggedFilterChange"
              @batchMarkRead="handleBatchMarkRead"
              @batchMarkUnread="handleBatchMarkUnread"
              @batchToggleStar="handleBatchToggleStar"
              @batchMoveTrash="handleBatchMoveTrash"
              @batchDelete="handleBatchDelete"
            />
            <div class="mail-content-pane">
              <MailContent
                key="mail-content"
                :mail="currentMailDetail"
                :loading="loadingDetail"
                :dynamicMailTheme="dynamicMailTheme"
                @reply="handleReply"
                @delete="handleDelete"
                @compose="currentView = 'compose'; replyToMail = null;"
                @markRead="handleMarkReadFromContent"
                @markUnread="handleMarkUnreadFromContent"
                @moveTrash="handleMoveTrashFromContent"
                @toggleStar="handleToggleStarFromContent"
                @forward="handleForward"
              />
            </div>
          </div>
        </Transition>
      </div>
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
import { ref, reactive, onMounted, computed, watch, shallowRef } from 'vue'
import SideNav from './components/SideNav.vue'
import MailList from './components/MailList.vue'
import MailContent from './components/MailContent.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import ComposeMail from './components/ComposeMail.vue'
import WindowControls from './components/WindowControls.vue'

const currentView = ref('inbox')
const lastMailView = ref('inbox')
const currentFolder = ref('INBOX')
const mails = ref([])
const mailPage = ref(1)
const mailTotal = ref(0)
const mailQuery = ref('')
const unreadOnly = ref(false)
const flaggedOnly = ref(false)
const MAIL_PAGE_SIZE = 20
const folders = ref([])
const selectedMailUid = ref(null)
const currentMailDetail = shallowRef(null)
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
const dynamicMailTheme = ref(true)
const connectingIds = ref(new Set())
let refreshGeneration = 0
let detailGeneration = 0

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
    return accounts.value.some(a => a.enabled !== false && a.connected)
  }
  return currentAccount.value?.enabled !== false && (currentAccount.value?.connected || false)
})

// 检测当前运行环境
const isTauri = typeof window !== 'undefined' && window.__IS_TAURI__
const isElectron = typeof window !== 'undefined' && window.electronAPI && !isTauri
const isDesktop = isTauri || isElectron

import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification'

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
  // 检查并请求通知权限（针对 Tauri v2）
  if (isTauri) {
    try {
      let permission = await isPermissionGranted();
      if (!permission) {
        permission = await requestPermission();
      }
      console.log('[Notification] Permission state:', permission);
    } catch (err) {
      console.error('[Notification] Failed to check permission:', err);
    }
  }

  const savedAccounts = localStorage.getItem('4ymail-accounts') || localStorage.getItem('forymail-accounts')
  if (savedAccounts) {
    try {
      accounts.value = JSON.parse(savedAccounts).map(a => ({ ...a, enabled: a.enabled !== false, unreadCount: 0, connected: false, folders: [] }))
    } catch {}
  }

  const savedSettings = localStorage.getItem('4ymail-settings') || localStorage.getItem('forymail-settings')
  if (savedSettings) {
    try {
      const settings = JSON.parse(savedSettings)
      pollingInterval.value = settings.pollingInterval || 5
      zoomLevel.value = settings.zoomLevel || 100
      dynamicMailTheme.value = settings.dynamicMailTheme !== undefined ? settings.dynamicMailTheme : true
      if (isDesktop) {
        window.electronAPI.updatePollingInterval(pollingInterval.value)
        if (isElectron) {
          window.electronAPI.setZoomFactor(zoomLevel.value / 100)
        }
      }
    } catch {}
  }

  // 初始化自启动状态
  if (isDesktop) {
    const asRes = await window.electronAPI.getAutostart()
    if (asRes.success) autoStart.value = asRes.enabled
  }

  // 监听来自主进程的连接异常
  if (isDesktop) {
    window.electronAPI.onConnectionError(({ accountId, error }) => {
      console.error(`[IPC] Connection Error for ${accountId}:`, error);
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
    window.electronAPI.onConnectionSuccess(async ({ accountId }) => {
      console.log(`[IPC] Connection Success for ${accountId}`);
      const acc = accounts.value.find(a => a.id === accountId)
      if (acc && acc.enabled !== false) {
        acc.connected = true
        showNotif(`账号 ${accountId} 已自动重新连接`, 'success')
        // 获取文件夹并刷新
        const fRes = await window.electronAPI.getFolders(acc.id)
        if (fRes.success) {
          acc.folders = fRes.data
          if (currentAccountId.value === acc.id) {
            folders.value = acc.folders
          }
        }
        if (currentAccountId.value === acc.id || currentAccountId.value === 'unified') {
          handleRefresh()
        }
      }
    })
    window.electronAPI.onNewMailArrived(({ accountId }) => {
      console.log(`[IPC] New Mail Arrived for ${accountId}`);
      // 如果当前在收件箱查看，且涉及该账号或统一视图，则刷新列表
      if (currentFolder.value === 'INBOX' && (currentAccountId.value === 'unified' || currentAccountId.value === accountId)) {
        handleRefresh()
      }
    })
    if (typeof window.electronAPI.onUnreadCount === 'function') {
      window.electronAPI.onUnreadCount(({ accountId, count }) => {
        const acc = accounts.value.find(a => a.id === accountId)
        if (acc) acc.unreadCount = Number(count) || 0
      })
    }
    window.electronAPI.onNotifMessage(({ message, type }) => {
      showNotif(message, type)
    })
    window.electronAPI.onRefreshAll(() => {
      handleRefresh()
    })
    window.electronAPI.onPlaySound(async () => {
      try {
        console.log('[Audio] Requesting sound data...');
        const soundData = await window.electronAPI.getSoundData();
        if (soundData) {
          console.log('[Audio] Received sound data, length:', soundData.length);
          const audio = new Audio(soundData);
          await audio.play();
          console.log('[Audio] Play success');
        }
      } catch (err) {
        console.warn('[Audio] Play failed:', err);
        // Fallback to relative path if IPC fails
        new Audio('sounds/message-sound.mp3').play().catch(() => {});
      }
    })

    if (window.electronAPI.onNotificationClicked) {
      window.electronAPI.onNotificationClicked(async (payload) => {
        console.log('[Notification Clicked] Navigating with payload:', payload);
        const { accountId, uid, folder } = payload || {};
        
        currentView.value = 'inbox';
        lastMailView.value = 'inbox';

        if (accountId && accounts.value.some(a => a.id === accountId)) {
          currentAccountId.value = accountId;
        }

        currentFolder.value = folder || 'INBOX';
        await handleRefresh();

        if (uid) {
          selectedMailUid.value = uid;
          const found = mails.value.find(m => m.uid === uid);
          if (found) {
            await handleSelectMail(found);
          } else {
            await handleSelectMail({ uid, accountId: accountId || currentAccountId.value });
          }
        }
      });
    }
  }
 
  // 自动连接所有已保存的账号
  setTimeout(async () => {
    console.log('[AutoConnect] Starting. Accounts found:', accounts.value.length)
    for (const acc of accounts.value) {
      if (acc.enabled !== false && acc.imap.host && acc.imap.user && !acc.connected) {
        console.log('[AutoConnect] Connecting:', acc.id)
        await handleConnect(acc.id)
      }
    }
  }, 2000) // 增加到 2 秒，确保一切就绪
})

// 监听账号变化持久化
watch(accounts, (newVal) => {
  const toSave = newVal.map(a => {
    const { connected, folders, unreadCount, ...rest } = a
    return rest
  })
  localStorage.setItem('4ymail-accounts', JSON.stringify(toSave))
}, { deep: true })

watch(pollingInterval, (newVal) => {
  saveSettings()
  if (isDesktop) window.electronAPI.updatePollingInterval(newVal)
})

// 动态计算根容器样式，实现全局缩放 (Tauri WebKit 兼容方案)
const rootStyle = computed(() => {
  const scale = zoomLevel.value / 100
  return {
    transform: scale === 1 ? 'none' : `scale(${scale})`,
    transformOrigin: scale === 1 ? undefined : 'top left',
    width: `${100 / scale}%`,
    height: `${100 / scale}%`
  }
})

watch(zoomLevel, (newVal) => {
  saveSettings()
  if (isElectron) {
    window.electronAPI.setZoomFactor(newVal / 100)
  }
})

watch(dynamicMailTheme, () => {
  saveSettings()
})

watch(autoStart, async (newVal) => {
  if (isDesktop) {
    try {
      await window.electronAPI.setAutostart(newVal)
    } catch (err) {
      console.error('Set autostart failed:', err)
    }
  }
})

function saveSettings() {
  localStorage.setItem('4ymail-settings', JSON.stringify({
    pollingInterval: pollingInterval.value,
    zoomLevel: zoomLevel.value,
    dynamicMailTheme: dynamicMailTheme.value
  }))
}

// 切换账号时自动加载文件夹和邮件
watch(currentAccountId, async (newId) => {
  refreshGeneration++
  detailGeneration++
  loadingList.value = false
  loadingDetail.value = false
  mailPage.value = 1
  mailTotal.value = 0
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
  } else {
    mails.value = []
    mailTotal.value = 0
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
  mailPage.value = 1
  mailTotal.value = 0
  selectedMailUid.value = null
  currentMailDetail.value = null
  mails.value = [] // 切换文件夹时清空
  unreadOnly.value = false
  flaggedOnly.value = false
  refreshGeneration++
  detailGeneration++
  loadingList.value = false
  loadingDetail.value = false
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
  accounts.value.push({ ...acc, enabled: acc.enabled !== false, unreadCount: 0, connected: false, folders: [] })
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

async function handleToggleAccount(accountId, enable) {
  const acc = accounts.value.find(a => a.id === accountId)
  if (!acc) return

  if (enable) {
    acc.enabled = true
    await handleConnect(accountId)
    return
  }

  acc.enabled = false
  try {
    await window.electronAPI.disconnect(accountId)
  } catch (err) {
    showNotif(`停用账号失败: ${err.message || err}`, 'error')
    acc.enabled = true
    return
  }

  acc.connected = false
  acc.folders = []
  acc.unreadCount = 0
  mails.value = mails.value.filter(mail => mail.accountId !== accountId)
  if (currentMailDetail.value?.accountId === accountId) {
    selectedMailUid.value = null
    currentMailDetail.value = null
  }
  if (currentAccountId.value === accountId) {
    folders.value = []
  }
  showNotif(`账号 ${accountId} 已停用`, 'info')
  if (currentAccountId.value === 'unified') await handleRefresh()
}


async function handleConnect(accountId) {
  if (!isDesktop) {
    showNotif('请在桌面应用中使用邮件功能', 'warning')
    return
  }
  const acc = accounts.value.find(a => a.id === accountId)
  if (!acc) return
  if (acc.enabled === false) return

  if (connectingIds.value.has(accountId)) return

  try {
    connectingIds.value.add(accountId)

    if (acc.connected) {
      try {
        await window.electronAPI.disconnect(acc.id)
      } catch {}
      acc.connected = false
    }

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
  }
}

async function handleDisconnect(accountId) {
  if (!isDesktop) return
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
  if (!isDesktop) return
  const generation = ++refreshGeneration

  // 先尝试重连断开的账号
  const disconnected = accounts.value.filter(a => a.enabled !== false && !a.connected && a.imap.host && a.imap.user)
  if (disconnected.length > 0) {
    for (const acc of disconnected) {
      try {
        const res = await window.electronAPI.connect({
          imap: toPlain(acc.imap),
          smtp: toPlain(acc.smtp)
        })
        if (generation !== refreshGeneration) return
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

  if (generation !== refreshGeneration) return

  if (!connected.value) {
    mails.value = []
    mailTotal.value = 0
    loadingList.value = false
    return
  }

  loadingList.value = true
  try {
    if (currentAccountId.value === 'unified') {
      const allMails = []
      let total = 0
      const needed = mailPage.value * MAIL_PAGE_SIZE
      for (const acc of accounts.value) {
        if (acc.enabled === false || !acc.connected) continue
        const realFolder = resolveFolder(currentFolder.value, acc.folders || [])
        const batchSize = Math.min(200, needed)
        const batchCount = Math.ceil(needed / batchSize)
        for (let batchPage = 1; batchPage <= batchCount; batchPage++) {
          const res = await window.electronAPI.getMessages({
            accountId: acc.id,
            folder: realFolder,
            page: batchPage,
            pageSize: batchSize,
            query: mailQuery.value || null,
            unreadOnly: unreadOnly.value,
            flaggedOnly: flaggedOnly.value,
          })
          if (generation !== refreshGeneration) return
          if (res.success) {
            if (batchPage === 1) {
              total += Number(res.data.total) || 0
              updateInboxUnreadCount(acc, realFolder, res.data.unreadCount)
            }
            allMails.push(...res.data.messages.map(m => ({ ...m, accountId: acc.id })))
            if (res.data.messages.length < batchSize) break
          } else {
            if (res.error && (res.error.includes('未连接') || res.error.includes('断开'))) acc.connected = false
            else showNotif(`加载 ${acc.id} 邮件失败: ${res.error}`, 'error')
            break
          }
        }
      }
      allMails.sort((a, b) => (new Date(b.date).getTime() || 0) - (new Date(a.date).getTime() || 0))
      const lastPage = Math.max(1, Math.ceil(total / MAIL_PAGE_SIZE))
      if (mailPage.value > lastPage) {
        mailPage.value = lastPage
        await handleRefresh()
        return
      }
      const offset = (mailPage.value - 1) * MAIL_PAGE_SIZE
      mails.value = allMails.slice(offset, offset + MAIL_PAGE_SIZE)
      mailTotal.value = total
    } else {
      const acc = accounts.value.find(a => a.id === currentAccountId.value)
      const realFolder = resolveFolder(currentFolder.value, acc?.folders || [])
      const res = await window.electronAPI.getMessages({
        accountId: currentAccountId.value,
        folder: realFolder,
        page: mailPage.value,
        pageSize: MAIL_PAGE_SIZE,
        query: mailQuery.value || null,
        unreadOnly: unreadOnly.value,
        flaggedOnly: flaggedOnly.value,
      })
      if (generation !== refreshGeneration) return
      if (res.success) {
        const total = Number(res.data.total) || 0
        const lastPage = Math.max(1, Math.ceil(total / MAIL_PAGE_SIZE))
        if (mailPage.value > lastPage) {
          mailPage.value = lastPage
          await handleRefresh()
          return
        }
        if (acc) updateInboxUnreadCount(acc, realFolder, res.data.unreadCount)
        mails.value = res.data.messages.map(m => ({ ...m, accountId: currentAccountId.value }))
        mailTotal.value = total
      } else if (res.error && (res.error.includes('未连接') || res.error.includes('断开'))) {
        if (acc) acc.connected = false
        showNotif(`账号 ${currentAccountId.value} 连接已断开，正在尝试重连...`, 'warning')
        if (acc && acc.imap.host && acc.imap.user) {
          try {
            const reconnectRes = await window.electronAPI.connect({
              imap: toPlain(acc.imap),
              smtp: toPlain(acc.smtp)
            })
            if (reconnectRes.success) {
              acc.connected = true
              const fRes = await window.electronAPI.getFolders(acc.id)
              if (fRes.success) {
                acc.folders = fRes.data
                if (currentAccountId.value === acc.id) {
                  folders.value = acc.folders
                }
              }
              showNotif(`账号 ${acc.id} 已重新连接`, 'success')
              const retryRes = await window.electronAPI.getMessages({
                accountId: currentAccountId.value,
                folder: realFolder,
                page: mailPage.value,
                pageSize: MAIL_PAGE_SIZE,
                query: mailQuery.value || null,
                unreadOnly: unreadOnly.value,
                flaggedOnly: flaggedOnly.value,
              })
              if (generation !== refreshGeneration) return
              if (retryRes.success) {
                mails.value = retryRes.data.messages.map(m => ({ ...m, accountId: currentAccountId.value }))
                mailTotal.value = Number(retryRes.data.total) || 0
              }
            } else {
              showNotif(`重连失败: ${reconnectRes.error}`, 'error')
            }
          } catch (err) {
            showNotif(`重连失败: ${err.message}`, 'error')
          }
        }
      } else {
        showNotif(`加载邮件失败: ${res.error}`, 'error')
      }
    }
  } catch (err) {
    if (generation === refreshGeneration) showNotif('刷新失败: ' + err.message, 'error')
  } finally {
    if (generation === refreshGeneration) loadingList.value = false
  }
}

async function handleChangeFolder(folder) {
  detailGeneration++
  loadingDetail.value = false
  currentFolder.value = folder
  mailPage.value = 1
  mailTotal.value = 0
  unreadOnly.value = false
  flaggedOnly.value = false
  selectedMailUid.value = null
  currentMailDetail.value = null
  await handleRefresh()
}

function handlePageChange(page) {
  detailGeneration++
  loadingDetail.value = false
  mailPage.value = Math.max(1, page)
  selectedMailUid.value = null
  currentMailDetail.value = null
  handleRefresh()
}

function handleSearchChange(query) {
  detailGeneration++
  loadingDetail.value = false
  mailQuery.value = query
  mailPage.value = 1
  selectedMailUid.value = null
  currentMailDetail.value = null
  handleRefresh()
}

function handleUnreadFilterChange(enabled) {
  detailGeneration++
  loadingDetail.value = false
  unreadOnly.value = enabled
  mailPage.value = 1
  selectedMailUid.value = null
  currentMailDetail.value = null
  handleRefresh()
}

function handleFlaggedFilterChange(enabled) {
  detailGeneration++
  loadingDetail.value = false
  flaggedOnly.value = enabled
  mailPage.value = 1
  selectedMailUid.value = null
  currentMailDetail.value = null
  handleRefresh()
}

async function handleSelectMail(mail) {
  // 现在接收的是 mail 对象
  const uid = mail.uid
  const accountId = mail.accountId || currentAccountId.value
  
  selectedMailUid.value = uid
  if (!isDesktop) return
  const generation = ++detailGeneration
  
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
    if (generation !== detailGeneration) return
    if (res.success) {
      currentMailDetail.value = { ...res.data, accountId, folder: realFolder }
      // 更新列表中的已读状态
      const idx = mails.value.findIndex(m => m.uid === uid && (m.accountId === accountId))
      if (idx !== -1 && !mails.value[idx].seen) {
        mails.value[idx].seen = true
        adjustUnreadCount(accountId, -1, realFolder)
      }
    } else {
      showNotif('加载邮件失败: ' + res.error, 'error')
    }
  } catch (err) {
    if (generation !== detailGeneration) return
    showNotif('加载邮件失败: ' + err.message, 'error')
  } finally {
    if (generation === detailGeneration) loadingDetail.value = false
  }
}

function isInboxFolder(account, folder) {
  return folder === 'INBOX' || account?.folders?.some(f => f.path === folder && f.specialUse === '\\Inbox')
}

function updateInboxUnreadCount(account, folder, count) {
  if (isInboxFolder(account, folder)) account.unreadCount = Number(count) || 0
}

function adjustUnreadCount(accountId, delta, folder) {
  const acc = accounts.value.find(a => a.id === accountId)
  if (acc && isInboxFolder(acc, folder)) {
    acc.unreadCount = Math.max(0, (Number(acc.unreadCount) || 0) + delta)
  }
}

function removeMailFromList(uid, accountId, folder) {
  const mail = mails.value.find(item => item.uid === uid && item.accountId === accountId)
  if (mail && !mail.seen) adjustUnreadCount(accountId, -1, folder)
  mails.value = mails.value.filter(item => !(item.uid === uid && item.accountId === accountId))
}

function handleReply(mail) {
  replyToMail.value = mail
  currentView.value = 'compose'
}

function handleForward(mail) {
  // 转发：设置 replyToMail 但标记为转发模式
  replyToMail.value = { ...mail, isForward: true }
  currentView.value = 'compose'
}

// 从邮件详情页标记已读
async function handleMarkReadFromContent(mail) {
  if (!isDesktop || !mail) return
  const accountId = mail.accountId || currentAccountId.value
  if (accountId === 'unified') return
  const acc = accounts.value.find(a => a.id === accountId)
  const folder = resolveFolder(currentFolder.value, acc?.folders || [])
  try {
    const res = await window.electronAPI.markSeen({ accountId, folder, uid: mail.uid })
    if (res.success) {
      const idx = mails.value.findIndex(m => m.uid === mail.uid && m.accountId === accountId)
      if (idx !== -1 && !mails.value[idx].seen) {
        mails.value[idx].seen = true
        adjustUnreadCount(accountId, -1, folder)
      }
      if (currentMailDetail.value) currentMailDetail.value.seen = true
      showNotif('已标记为已读', 'success')
    }
  } catch (err) {
    showNotif('操作失败: ' + err.message, 'error')
  }
}

// 从邮件详情页标记未读
async function handleMarkUnreadFromContent(mail) {
  if (!isDesktop || !mail) return
  const accountId = mail.accountId || currentAccountId.value
  if (accountId === 'unified') return
  const acc = accounts.value.find(a => a.id === accountId)
  const folder = resolveFolder(currentFolder.value, acc?.folders || [])
  try {
    const res = await window.electronAPI.markUnseen({ accountId, folder, uid: mail.uid })
    if (res.success) {
      const idx = mails.value.findIndex(m => m.uid === mail.uid && m.accountId === accountId)
      if (idx !== -1 && mails.value[idx].seen) {
        mails.value[idx].seen = false
        adjustUnreadCount(accountId, 1, folder)
      }
      if (currentMailDetail.value) currentMailDetail.value.seen = false
      showNotif('已标记为未读', 'success')
    }
  } catch (err) {
    showNotif('操作失败: ' + err.message, 'error')
  }
}

// 从邮件详情页移至垃圾箱
async function handleMoveTrashFromContent(mail) {
  if (!isDesktop || !mail) return
  const accountId = mail.accountId || currentAccountId.value
  if (accountId === 'unified') return
  const acc = accounts.value.find(a => a.id === accountId)
  const folder = resolveFolder(currentFolder.value, acc?.folders || [])
  const trashFolder = resolveFolder('Deleted Messages', acc?.folders || [])
  try {
    const res = await window.electronAPI.moveMail({ accountId, folder, uid: mail.uid, destination: trashFolder })
    if (res.success) {
      removeMailFromList(mail.uid, accountId, folder)
      selectedMailUid.value = null
      currentMailDetail.value = null
      showNotif('已移至垃圾箱', 'success')
    } else {
      showNotif('移动失败: ' + res.error, 'error')
    }
  } catch (err) {
    showNotif('移动失败: ' + err.message, 'error')
  }
}

// === 右键菜单处理函数 ===

async function handleMarkRead(mail) {
  if (!isDesktop) return
  const accountId = mail.accountId || currentAccountId.value
  const folder = currentAccountId.value === 'unified'
    ? resolveFolder(currentFolder.value, accounts.value.find(a => a.id === accountId)?.folders || [])
    : currentFolder.value
  try {
    const res = await window.electronAPI.markSeen({ accountId, folder, uid: mail.uid })
    if (res.success) {
      const idx = mails.value.findIndex(m => m.uid === mail.uid && m.accountId === accountId)
      if (idx !== -1 && !mails.value[idx].seen) {
        mails.value[idx].seen = true
        adjustUnreadCount(accountId, -1, folder)
      }
    }
  } catch (err) {
    showNotif('操作失败: ' + err.message, 'error')
  }
}

async function handleMarkUnread(mail) {
  if (!isDesktop) return
  const accountId = mail.accountId || currentAccountId.value
  const folder = currentAccountId.value === 'unified'
    ? resolveFolder(currentFolder.value, accounts.value.find(a => a.id === accountId)?.folders || [])
    : currentFolder.value
  try {
    const res = await window.electronAPI.markUnseen({ accountId, folder, uid: mail.uid })
    if (res.success) {
      const idx = mails.value.findIndex(m => m.uid === mail.uid && m.accountId === accountId)
      if (idx !== -1 && mails.value[idx].seen) {
        mails.value[idx].seen = false
        adjustUnreadCount(accountId, 1, folder)
      }
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
  if (!isDesktop) return
  const accountId = mail.accountId || currentAccountId.value
  if (accountId === 'unified') return
  const folder = currentAccountId.value === 'unified'
    ? resolveFolder(currentFolder.value, accounts.value.find(a => a.id === accountId)?.folders || [])
    : currentFolder.value
  try {
    const res = await window.electronAPI.deleteMail({ accountId, folder, uid: mail.uid })
    if (res.success) {
      removeMailFromList(mail.uid, accountId, folder)
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
  if (!isDesktop) return
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
      removeMailFromList(mail.uid, accountId, folder)
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
  if (!isDesktop) return
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
      removeMailFromList(uid, accountId, realFolder)
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

async function handleToggleStar(mail) {
  if (!isDesktop || !mail) return
  const accountId = mail.accountId || currentAccountId.value
  if (!accountId || accountId === 'unified') return
  const acc = accounts.value.find(a => a.id === accountId)
  const folder = resolveFolder(currentFolder.value, acc?.folders || [])
  const newFlagged = !mail.flagged

  try {
    const res = newFlagged
      ? await window.electronAPI.markFlagged({ accountId, folder, uids: [mail.uid] })
      : await window.electronAPI.markUnflagged({ accountId, folder, uids: [mail.uid] })

    if (res.success) {
      const idx = mails.value.findIndex(m => m.uid === mail.uid && m.accountId === accountId)
      if (idx !== -1) {
        mails.value[idx].flagged = newFlagged
      }
      if (currentMailDetail.value && currentMailDetail.value.uid === mail.uid && (currentMailDetail.value.accountId === accountId || !currentMailDetail.value.accountId)) {
        currentMailDetail.value.flagged = newFlagged
      }
      showNotif(newFlagged ? '已添加星标' : '已取消星标', 'success')
      if (flaggedOnly.value && !newFlagged) {
        mails.value = mails.value.filter(m => !(m.uid === mail.uid && m.accountId === accountId))
      }
    } else {
      showNotif('操作失败: ' + res.error, 'error')
    }
  } catch (err) {
    showNotif('操作失败: ' + err.message, 'error')
  }
}

async function handleToggleStarFromContent(mail) {
  await handleToggleStar(mail)
}

function groupMailsByAccount(targetMails) {
  const groups = new Map()
  for (const mail of targetMails) {
    const accId = mail.accountId || currentAccountId.value
    if (!accId || accId === 'unified') continue
    if (!groups.has(accId)) {
      groups.set(accId, [])
    }
    groups.get(accId).push(mail)
  }
  return groups
}

async function handleBatchMarkRead(targetMails) {
  if (!isDesktop || !targetMails || !targetMails.length) return
  const groups = groupMailsByAccount(targetMails)
  let successCount = 0

  for (const [accountId, items] of groups.entries()) {
    const acc = accounts.value.find(a => a.id === accountId)
    const folder = resolveFolder(currentFolder.value, acc?.folders || [])
    const uids = items.map(m => m.uid)

    try {
      const res = await window.electronAPI.batchMarkSeen({ accountId, folder, uids })
      if (res.success) {
        const uidSet = new Set(uids)
        let unreadDelta = 0
        for (const m of mails.value) {
          if (m.accountId === accountId && uidSet.has(m.uid)) {
            if (!m.seen) {
              m.seen = true
              unreadDelta--
            }
          }
        }
        if (unreadDelta !== 0) {
          adjustUnreadCount(accountId, unreadDelta, folder)
        }
        if (currentMailDetail.value && (currentMailDetail.value.accountId === accountId || !currentMailDetail.value.accountId) && uidSet.has(currentMailDetail.value.uid)) {
          currentMailDetail.value.seen = true
        }
        successCount += items.length
      } else {
        showNotif(`账号 ${accountId} 批量标为已读失败: ${res.error}`, 'error')
      }
    } catch (err) {
      showNotif(`批量标为已读失败: ${err.message}`, 'error')
    }
  }

  if (successCount > 0) {
    showNotif(`已将 ${successCount} 封邮件标记为已读`, 'success')
  }
}

async function handleBatchMarkUnread(targetMails) {
  if (!isDesktop || !targetMails || !targetMails.length) return
  const groups = groupMailsByAccount(targetMails)
  let successCount = 0

  for (const [accountId, items] of groups.entries()) {
    const acc = accounts.value.find(a => a.id === accountId)
    const folder = resolveFolder(currentFolder.value, acc?.folders || [])
    const uids = items.map(m => m.uid)

    try {
      const res = await window.electronAPI.batchMarkUnseen({ accountId, folder, uids })
      if (res.success) {
        const uidSet = new Set(uids)
        let unreadDelta = 0
        for (const m of mails.value) {
          if (m.accountId === accountId && uidSet.has(m.uid)) {
            if (m.seen) {
              m.seen = false
              unreadDelta++
            }
          }
        }
        if (unreadDelta !== 0) {
          adjustUnreadCount(accountId, unreadDelta, folder)
        }
        if (currentMailDetail.value && (currentMailDetail.value.accountId === accountId || !currentMailDetail.value.accountId) && uidSet.has(currentMailDetail.value.uid)) {
          currentMailDetail.value.seen = false
        }
        successCount += items.length
      } else {
        showNotif(`账号 ${accountId} 批量标为未读失败: ${res.error}`, 'error')
      }
    } catch (err) {
      showNotif(`批量标为未读失败: ${err.message}`, 'error')
    }
  }

  if (successCount > 0) {
    showNotif(`已将 ${successCount} 封邮件标记为未读`, 'success')
  }
}

async function handleBatchToggleStar(targetMails, targetFlagged) {
  if (!isDesktop || !targetMails || !targetMails.length) return
  const groups = groupMailsByAccount(targetMails)
  let successCount = 0

  for (const [accountId, items] of groups.entries()) {
    const acc = accounts.value.find(a => a.id === accountId)
    const folder = resolveFolder(currentFolder.value, acc?.folders || [])
    const uids = items.map(m => m.uid)

    try {
      const res = targetFlagged
        ? await window.electronAPI.markFlagged({ accountId, folder, uids })
        : await window.electronAPI.markUnflagged({ accountId, folder, uids })

      if (res.success) {
        const uidSet = new Set(uids)
        for (const m of mails.value) {
          if (m.accountId === accountId && uidSet.has(m.uid)) {
            m.flagged = targetFlagged
          }
        }
        if (currentMailDetail.value && (currentMailDetail.value.accountId === accountId || !currentMailDetail.value.accountId) && uidSet.has(currentMailDetail.value.uid)) {
          currentMailDetail.value.flagged = targetFlagged
        }
        successCount += items.length
      } else {
        showNotif(`账号 ${accountId} 批量星标操作失败: ${res.error}`, 'error')
      }
    } catch (err) {
      showNotif(`批量星标操作失败: ${err.message}`, 'error')
    }
  }

  if (successCount > 0) {
    showNotif(`已${targetFlagged ? '星标' : '取消星标'} ${successCount} 封邮件`, 'success')
    if (flaggedOnly.value && !targetFlagged) {
      await handleRefresh()
    }
  }
}

async function handleBatchMoveTrash(targetMails) {
  if (!isDesktop || !targetMails || !targetMails.length) return
  const groups = groupMailsByAccount(targetMails)
  let successCount = 0

  for (const [accountId, items] of groups.entries()) {
    const acc = accounts.value.find(a => a.id === accountId)
    const folder = resolveFolder(currentFolder.value, acc?.folders || [])
    const trashFolder = resolveFolder('Deleted Messages', acc?.folders || [])
    const uids = items.map(m => m.uid)

    try {
      const res = await window.electronAPI.batchMoveMail({
        accountId,
        folder,
        uids,
        destination: trashFolder,
      })
      if (res.success) {
        const uidSet = new Set(uids)
        for (const m of items) {
          removeMailFromList(m.uid, accountId, folder)
        }
        if (currentMailDetail.value && (currentMailDetail.value.accountId === accountId || !currentMailDetail.value.accountId) && uidSet.has(currentMailDetail.value.uid)) {
          selectedMailUid.value = null
          currentMailDetail.value = null
        }
        successCount += items.length
      } else {
        showNotif(`账号 ${accountId} 移至垃圾箱失败: ${res.error}`, 'error')
      }
    } catch (err) {
      showNotif(`批量移至垃圾箱失败: ${err.message}`, 'error')
    }
  }

  if (successCount > 0) {
    showNotif(`已将 ${successCount} 封邮件移至垃圾箱`, 'success')
  }
}

async function handleBatchDelete(targetMails) {
  if (!isDesktop || !targetMails || !targetMails.length) return
  const groups = groupMailsByAccount(targetMails)
  let successCount = 0

  for (const [accountId, items] of groups.entries()) {
    const acc = accounts.value.find(a => a.id === accountId)
    const folder = resolveFolder(currentFolder.value, acc?.folders || [])
    const uids = items.map(m => m.uid)

    try {
      const res = await window.electronAPI.batchDeleteMail({
        accountId,
        folder,
        uids,
      })
      if (res.success) {
        const uidSet = new Set(uids)
        for (const m of items) {
          removeMailFromList(m.uid, accountId, folder)
        }
        if (currentMailDetail.value && (currentMailDetail.value.accountId === accountId || !currentMailDetail.value.accountId) && uidSet.has(currentMailDetail.value.uid)) {
          selectedMailUid.value = null
          currentMailDetail.value = null
        }
        successCount += items.length
      } else {
        showNotif(`账号 ${accountId} 批量删除失败: ${res.error}`, 'error')
      }
    } catch (err) {
      showNotif(`批量删除失败: ${err.message}`, 'error')
    }
  }

  if (successCount > 0) {
    showNotif(`已彻底删除 ${successCount} 封邮件`, 'success')
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
  /* Keep the native drag surface away from the three window controls. */
  right: 126px;
  height: 32px;
  -webkit-app-region: drag; /* 兼容 Electron */
  z-index: 10;
}

.app-window-controls {
  position: absolute;
  top: 0;
  right: 0;
  z-index: 1001;
}

.app-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.app-view-host {
  position: relative;
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  isolation: isolate;
  /* 强制锁定独立的 GPU 合成上下文，防止 WebKitGTK 在动画结束移出 class 时发生图层注销降级（Layer De-promotion）导致过期双缓冲画面（Stale Frame）闪烁 */
  transform: translateZ(0);
  -webkit-transform: translateZ(0);
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
}

.app-view-host > * {
  transform: translateZ(0);
  -webkit-transform: translateZ(0);
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
}

.mail-content-pane {
  position: relative;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.mail-content-pane > .mailcontent {
  width: 100%;
  height: 100%;
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
