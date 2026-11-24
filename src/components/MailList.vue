<template>
  <div class="maillist">
    <div class="maillist-header">
      <div class="search-box">
        <svg class="search-icon" width="15" height="15" viewBox="0 0 24 24" fill="none">
          <circle cx="11" cy="11" r="8" stroke="currentColor" stroke-width="1.5"/>
          <path d="M21 21l-4.35-4.35" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <input
          v-model="searchInput"
          type="text"
          placeholder="搜索主题、发件人或收件人..."
          class="search-input"
        />
      </div>
      <div class="maillist-toolbar">
        <button
          class="toolbar-btn filter-btn unread-filter-btn"
          :class="{ active: unreadOnly }"
          @click="$emit('unread-change', !unreadOnly)"
          :title="unreadOnly ? '显示全部邮件' : '只显示未读邮件'"
        >未读</button>
        <button
          class="toolbar-btn filter-btn star-filter-btn"
          :class="{ active: flaggedOnly }"
          @click="$emit('flagged-change', !flaggedOnly)"
          :title="flaggedOnly ? '显示全部邮件' : '只显示星标邮件'"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" :fill="flaggedOnly ? 'currentColor' : 'none'">
            <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          <span>星标</span>
        </button>
        <button
          class="toolbar-btn select-mode-btn"
          :class="{ active: isSelectMode || checkedCount > 0 }"
          @click="toggleSelectMode"
          :title="isSelectMode ? '退出批量选择' : '批量选择邮件'"
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none">
            <rect x="3" y="5" width="5" height="5" rx="1" stroke="currentColor" stroke-width="1.6"/>
            <path d="M11 7.5h10M11 16.5h10" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>
            <rect x="3" y="14" width="5" height="5" rx="1" stroke="currentColor" stroke-width="1.6"/>
          </svg>
        </button>
        <button class="toolbar-btn" @click="$emit('refresh')" title="刷新">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" :class="{ spinning: loading }">
            <path d="M23 4v6h-6M1 20v-6h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </div>
      <div v-if="loading && mails.length" class="refresh-progress"><span></span></div>
    </div>

    <!-- 批量操作工具栏 -->
    <Transition name="batch-bar-anim">
      <div v-if="isSelectMode || checkedCount > 0" class="batch-toolbar">
        <div class="batch-left">
          <button class="batch-checkbox-btn" @click="toggleSelectAll" :title="isAllSelected ? '取消全选' : '全选当前页'">
            <span class="custom-checkbox" :class="{ checked: isAllSelected, partial: isPartialSelected }">
              <svg v-if="isAllSelected" width="10" height="10" viewBox="0 0 24 24" fill="none">
                <path d="M20 6L9 17L4 12" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span v-else-if="isPartialSelected" class="partial-dash"></span>
            </span>
          </button>
          <span class="batch-count-badge">已选 {{ checkedCount }} 封</span>
        </div>
        <div class="batch-actions">
          <button
            class="batch-action-btn"
            :disabled="checkedCount === 0"
            @click="triggerBatchMarkRead"
            title="标记为已读"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" stroke="currentColor" stroke-width="1.8"/>
              <circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.8"/>
            </svg>
          </button>
          <button
            class="batch-action-btn"
            :disabled="checkedCount === 0"
            @click="triggerBatchMarkUnread"
            title="标记为未读"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.8"/>
              <path d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <button
            class="batch-action-btn star-btn"
            :disabled="checkedCount === 0"
            @click="triggerBatchToggleStar"
            title="标记/取消星标"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <button
            class="batch-action-btn"
            :disabled="checkedCount === 0"
            @click="triggerBatchMoveTrash"
            title="移至垃圾箱"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path d="M3 6h18M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
            </svg>
          </button>
          <button
            class="batch-action-btn danger"
            :disabled="checkedCount === 0"
            @click="promptBatchDelete"
            title="彻底删除"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <div class="batch-divider"></div>
          <button
            class="batch-close-btn"
            @click="exitSelectMode"
            title="退出多选"
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none">
              <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/>
            </svg>
          </button>
        </div>
      </div>
    </Transition>

    <div class="maillist-body" ref="listRef">
      <Transition name="list-state" mode="out-in">
        <div v-if="loading && mails.length === 0" key="loading" class="maillist-empty">
          <div class="loading-spinner"></div>
          <span>正在加载...</span>
        </div>
        <div v-else-if="!loading && mails.length === 0" key="empty" class="maillist-empty">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" style="opacity:0.3">
            <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.5"/>
            <path d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          <span>{{ searchQuery || unreadOnly || flaggedOnly ? '没有匹配的邮件' : '没有邮件' }}</span>
        </div>
      </Transition>
      <TransitionGroup v-if="mails.length > 0" name="list" tag="div" class="mail-items-container">
        <div
          v-for="(mail, index) in mails"
          :key="`${mail.accountId || ''}:${mail.uid}`"
          class="mail-item"
          :class="{ selected: selectedMail === mail.uid, unread: !mail.seen, checked: isChecked(mail) }"
          @click="handleMailClick($event, mail, index)"
          @contextmenu.prevent="openContext($event, mail)"
        >
          <!-- 复选框 -->
          <div
            class="mail-checkbox-wrap"
            :class="{ visible: isSelectMode || isChecked(mail) }"
            @click.stop="toggleCheck(mail, $event, index)"
          >
            <span class="custom-checkbox" :class="{ checked: isChecked(mail) }">
              <svg v-if="isChecked(mail)" width="10" height="10" viewBox="0 0 24 24" fill="none">
                <path d="M20 6L9 17L4 12" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </span>
          </div>

          <div class="mail-info">
            <div class="mail-top">
              <div class="mail-sender-line">
                <span v-if="!mail.seen" class="unread-dot" aria-label="未读"></span>
                <span class="mail-sender">{{ mail.from?.name || mail.from?.address || '未知发件人' }}</span>
              </div>
              <div class="mail-top-right">
                <!-- 星标按钮 -->
                <button
                  class="mail-star-btn"
                  :class="{ active: mail.flagged }"
                  @click.stop="$emit('toggleStar', mail)"
                  :title="mail.flagged ? '取消星标' : '标记星标'"
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" :fill="mail.flagged ? '#f39c12' : 'none'">
                    <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" :stroke="mail.flagged ? '#f39c12' : 'currentColor'" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </button>
                <span class="mail-time">{{ formatTime(mail.date) }}</span>
              </div>
            </div>
            <div class="mail-subject" :class="{ 'font-bold': !mail.seen }">{{ mail.subject || '（无主题）' }}</div>
            <div v-if="mail.accountId" class="mail-meta">
              <span class="acc-label">{{ mail.accountId }}</span>
            </div>
          </div>
        </div>
      </TransitionGroup>
    </div>

    <div class="mail-list-footer">
      <span>{{ total }} 封{{ searchQuery ? '匹配邮件' : (unreadOnly ? '未读邮件' : (flaggedOnly ? '星标邮件' : '邮件')) }}</span>
      <div class="page-controls">
        <button class="toolbar-btn" :disabled="page <= 1 || loading" @click="$emit('page-change', page - 1)" title="上一页">‹</button>
        <span>{{ page }} / {{ pageCount }}</span>
        <button class="toolbar-btn" :disabled="page >= pageCount || loading" @click="$emit('page-change', page + 1)" title="下一页">›</button>
      </div>
    </div>

    <!-- 彻底删除确认弹窗 -->
    <teleport to="body">
      <transition name="fade">
        <div v-if="showDeleteConfirm" class="confirm-modal-overlay" @click="showDeleteConfirm = false">
          <div class="confirm-modal" @click.stop>
            <div class="confirm-icon danger">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                <path d="M12 9v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
              </svg>
            </div>
            <h3 class="confirm-title">确认彻底删除？</h3>
            <p class="confirm-desc">即将彻底删除选中的 {{ checkedCount }} 封邮件，此操作无法恢复。</p>
            <div class="confirm-buttons">
              <button class="confirm-btn cancel" @click="showDeleteConfirm = false">取消</button>
              <button class="confirm-btn danger" @click="executeBatchDelete">彻底删除</button>
            </div>
          </div>
        </div>
      </transition>
    </teleport>

    <!-- 右键菜单 -->
    <teleport to="body">
      <transition name="ctx-pop">
        <div
          v-if="showContext"
          class="context-menu"
          :style="{ left: contextPos.x + 'px', top: contextPos.y + 'px' }"
          ref="ctxMenuRef"
        >
          <!-- 批量操作菜单（选了多封且右击项已被勾选） -->
          <template v-if="isBatchContext">
            <div class="ctx-header">已选择 {{ checkedCount }} 封邮件</div>
            <div class="ctx-item" @click="ctxBatchAction('markRead')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" stroke="currentColor" stroke-width="1.5"/>
                <circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>标记为已读</span>
            </div>
            <div class="ctx-item" @click="ctxBatchAction('markUnread')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.5"/>
                <path d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>标记为未读</span>
            </div>
            <div class="ctx-item" @click="ctxBatchAction('toggleStar')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>添加/取消星标</span>
            </div>
            <div class="ctx-divider"></div>
            <div class="ctx-item" @click="ctxBatchAction('moveTrash')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M3 6h18M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <span>移至垃圾箱</span>
            </div>
            <div class="ctx-item danger" @click="ctxBatchAction('delete')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <span>彻底删除</span>
            </div>
            <div class="ctx-divider"></div>
            <div class="ctx-item" @click="clearSelection">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
              <span>取消选择</span>
            </div>
          </template>

          <!-- 单封邮件菜单 -->
          <template v-else>
            <div class="ctx-item" @click="ctxAction('toggleRead')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path v-if="contextMail?.seen" d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" stroke="currentColor" stroke-width="1.5"/>
                <circle v-if="contextMail?.seen" cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.5"/>
                <path v-if="!contextMail?.seen" d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.5"/>
                <path v-if="!contextMail?.seen" d="M22 6l-10 7L2 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>{{ contextMail?.seen ? '标记为未读' : '标记为已读' }}</span>
            </div>
            <div class="ctx-item" @click="ctxAction('toggleStar')">
              <svg width="14" height="14" viewBox="0 0 24 24" :fill="contextMail?.flagged ? '#f39c12' : 'none'">
                <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" :stroke="contextMail?.flagged ? '#f39c12' : 'currentColor'" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>{{ contextMail?.flagged ? '取消星标' : '标记星标' }}</span>
            </div>
            <div class="ctx-item" @click="ctxAction('reply')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <path d="M9 17l-5-5 5-5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M4 12h12a4 4 0 014 4v1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>回复</span>
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
            <div class="ctx-item" @click="ctxAction('select')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <rect x="3" y="3" width="18" height="18" rx="2" stroke="currentColor" stroke-width="1.5"/>
                <path d="M9 12l2 2 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              <span>多选</span>
            </div>
            <div class="ctx-item" @click="ctxAction('copyAddr')">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                <rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="1.5"/>
                <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>复制发件人地址</span>
            </div>
          </template>
        </div>
      </transition>
    </teleport>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import GlassSelect from './GlassSelect.vue'

const props = defineProps({
  mails: Array,
  selectedMail: [Number, String, null],
  currentFolder: String,
  loading: Boolean,
  folders: Array,
  page: { type: Number, default: 1 },
  total: { type: Number, default: 0 },
  query: { type: String, default: '' },
  unreadOnly: { type: Boolean, default: false },
  flaggedOnly: { type: Boolean, default: false },
})

const emit = defineEmits([
  'select',
  'refresh',
  'changeFolder',
  'markRead',
  'markUnread',
  'toggleStar',
  'reply',
  'delete',
  'moveTrash',
  'copyAddr',
  'page-change',
  'search-change',
  'unread-change',
  'flagged-change',
  'batchMarkRead',
  'batchMarkUnread',
  'batchToggleStar',
  'batchMoveTrash',
  'batchDelete',
])

const searchInput = ref(props.query)
let searchTimer
const showContext = ref(false)
const contextMail = ref(null)
const contextPos = ref({ x: 0, y: 0 })
const ctxMenuRef = ref(null)
const showDeleteConfirm = ref(false)

// 多选与批量操作状态
const isSelectMode = ref(false)
const selectedKeys = ref(new Set())
let lastClickedIndex = null

const getMailKey = (mail) => `${mail.accountId || ''}:${mail.uid}`

const checkedCount = computed(() => selectedKeys.value.size)
const mails = computed(() => props.mails || [])

const isAllSelected = computed(() => {
  return mails.value.length > 0 && mails.value.every(m => selectedKeys.value.has(getMailKey(m)))
})

const isPartialSelected = computed(() => {
  return !isAllSelected.value && mails.value.some(m => selectedKeys.value.has(getMailKey(m)))
})

const selectedMailObjects = computed(() => {
  return mails.value.filter(m => selectedKeys.value.has(getMailKey(m)))
})

const isBatchContext = computed(() => {
  return showContext.value && contextMail.value && selectedKeys.value.has(getMailKey(contextMail.value)) && selectedKeys.value.size > 1
})

function isChecked(mail) {
  return selectedKeys.value.has(getMailKey(mail))
}

function toggleSelectMode() {
  isSelectMode.value = !isSelectMode.value
  if (!isSelectMode.value) {
    selectedKeys.value.clear()
    lastClickedIndex = null
  }
}

function exitSelectMode() {
  isSelectMode.value = false
  selectedKeys.value.clear()
  lastClickedIndex = null
}

function clearSelection() {
  selectedKeys.value.clear()
  showContext.value = false
}

function toggleSelectAll() {
  if (isAllSelected.value) {
    mails.value.forEach(m => selectedKeys.value.delete(getMailKey(m)))
  } else {
    mails.value.forEach(m => selectedKeys.value.add(getMailKey(m)))
    isSelectMode.value = true
  }
}

function toggleCheck(mail, event, index) {
  const key = getMailKey(mail)
  if (event?.shiftKey && lastClickedIndex !== null && index !== undefined) {
    const start = Math.min(lastClickedIndex, index)
    const end = Math.max(lastClickedIndex, index)
    for (let i = start; i <= end; i++) {
      if (mails.value[i]) selectedKeys.value.add(getMailKey(mails.value[i]))
    }
  } else {
    if (selectedKeys.value.has(key)) {
      selectedKeys.value.delete(key)
    } else {
      selectedKeys.value.add(key)
    }
  }
  lastClickedIndex = index
  if (selectedKeys.value.size > 0) isSelectMode.value = true
}

function handleMailClick(event, mail, index) {
  if (isSelectMode.value) {
    toggleCheck(mail, event, index)
  } else if (event.ctrlKey || event.metaKey) {
    isSelectMode.value = true
    toggleCheck(mail, event, index)
  } else {
    emit('select', mail)
  }
}

function triggerBatchMarkRead() {
  if (selectedMailObjects.value.length === 0) return
  emit('batchMarkRead', [...selectedMailObjects.value])
  exitSelectMode()
}

function triggerBatchMarkUnread() {
  if (selectedMailObjects.value.length === 0) return
  emit('batchMarkUnread', [...selectedMailObjects.value])
  exitSelectMode()
}

function triggerBatchToggleStar() {
  if (selectedMailObjects.value.length === 0) return
  const allFlagged = selectedMailObjects.value.every(m => m.flagged)
  emit('batchToggleStar', [...selectedMailObjects.value], !allFlagged)
  exitSelectMode()
}

function triggerBatchMoveTrash() {
  if (selectedMailObjects.value.length === 0) return
  emit('batchMoveTrash', [...selectedMailObjects.value])
  exitSelectMode()
}

function promptBatchDelete() {
  if (selectedMailObjects.value.length === 0) return
  showContext.value = false
  showDeleteConfirm.value = true
}

function executeBatchDelete() {
  showDeleteConfirm.value = false
  if (selectedMailObjects.value.length === 0) return
  emit('batchDelete', [...selectedMailObjects.value])
  exitSelectMode()
}

function ctxBatchAction(action) {
  showContext.value = false
  switch (action) {
    case 'markRead':
      triggerBatchMarkRead()
      break
    case 'markUnread':
      triggerBatchMarkUnread()
      break
    case 'toggleStar':
      triggerBatchToggleStar()
      break
    case 'moveTrash':
      triggerBatchMoveTrash()
      break
    case 'delete':
      promptBatchDelete()
      break
  }
}

function handleKeyDown(e) {
  if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable)) return
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a') {
    e.preventDefault()
    toggleSelectAll()
  } else if (e.key === 'Escape') {
    if (showDeleteConfirm.value) {
      showDeleteConfirm.value = false
    } else if (isSelectMode.value || checkedCount.value > 0) {
      exitSelectMode()
    }
  } else if (e.key === 'Delete' || e.key === 'Backspace') {
    if (checkedCount.value > 0) {
      e.preventDefault()
      triggerBatchMoveTrash()
    }
  }
}

function openContext(event, mail) {
  showContext.value = false
  contextMail.value = mail
  const menuW = 200, menuH = 280
  let x = event.clientX
  let y = event.clientY
  if (x + menuW > window.innerWidth) x = window.innerWidth - menuW - 8
  if (y + menuH > window.innerHeight) y = window.innerHeight - menuH - 8
  contextPos.value = { x, y }
  requestAnimationFrame(() => {
    showContext.value = true
  })
}

function ctxAction(action) {
  if (!contextMail.value) return
  const mail = contextMail.value
  switch (action) {
    case 'toggleRead':
      if (mail.seen) {
        emit('markUnread', mail)
      } else {
        emit('markRead', mail)
      }
      break
    case 'toggleStar':
      emit('toggleStar', mail)
      break
    case 'select':
      selectedKeys.value.add(getMailKey(mail))
      isSelectMode.value = true
      break
    case 'reply':
      emit('select', mail)
      setTimeout(() => emit('reply', mail), 100)
      break
    case 'delete':
      emit('delete', mail)
      break
    case 'moveTrash':
      emit('moveTrash', mail)
      break
    case 'copyAddr':
      emit('copyAddr', mail)
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
  window.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  document.removeEventListener('mousedown', closeContext)
  window.removeEventListener('keydown', handleKeyDown)
  clearTimeout(searchTimer)
})

watch([() => props.currentFolder, () => props.page], () => {
  selectedKeys.value.clear()
  isSelectMode.value = false
  lastClickedIndex = null
})

const searchQuery = computed(() => props.query.trim())
const unreadOnly = computed(() => props.unreadOnly)
const flaggedOnly = computed(() => props.flaggedOnly)
const pageCount = computed(() => Math.max(1, Math.ceil(props.total / 20)))

watch(searchInput, value => {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(() => emit('search-change', value.trim()), 300)
})

watch(() => props.query, value => {
  if (searchInput.value !== value) searchInput.value = value
})

function getFolderName(folder) {
  const map = {
    '\\Inbox': '收件箱',
    '\\Sent': '已发送',
    '\\Drafts': '草稿箱',
    '\\Trash': '垃圾箱',
    '\\Junk': '垃圾邮件',
    '\\Archive': '归档',
  }
  if (folder.specialUse && map[folder.specialUse]) return map[folder.specialUse]
  return folder.name
}

function formatTime(dateStr) {
  if (!dateStr) return ''
  const date = new Date(dateStr)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const oneDay = 86400000

  if (diff < oneDay && date.getDate() === now.getDate()) {
    return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  }
  if (diff < oneDay * 2) return '昨天'
  if (diff < oneDay * 7) {
    const days = ['周日', '周一', '周二', '周三', '周四', '周五', '周六']
    return days[date.getDay()]
  }
  return date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })
}
</script>

<style scoped>
.maillist {
  width: var(--maillist-width);
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-maillist);
  border-right: 1px solid var(--glass-border);
  flex-shrink: 0;
  transition: background var(--transition-normal);
}

.maillist-header {
  position: relative;
  padding: 12px;
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
}

.refresh-progress {
  position: absolute;
  z-index: 2;
  left: 0;
  right: 0;
  bottom: -1px;
  height: 2px;
  overflow: hidden;
  pointer-events: none;
}

.refresh-progress span {
  display: block;
  width: 32%;
  height: 100%;
  background: var(--primary-light);
  animation: refresh-progress 1.1s ease-in-out infinite;
}

@keyframes refresh-progress {
  from { transform: translateX(-110%); }
  to { transform: translateX(420%); }
}

.search-box {
  position: relative;
  margin-bottom: 8px;
}

.search-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-tertiary);
}

.search-input {
  width: 100%;
  padding: 8px 12px 8px 34px;
  background: var(--input-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  color: var(--text-primary);
  font-size: 13px;
}

.search-input:focus {
  border-color: var(--primary);
}

.maillist-toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

.folder-select {
  flex: 1;
  padding: 5px 8px;
  background: var(--input-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 12px;
  font-family: inherit;
  outline: none;
  cursor: pointer;
}

.folder-select option {
  background: var(--select-bg);
  color: var(--text-primary);
}

.toolbar-btn {
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
}

.toolbar-btn:hover {
  background: var(--glass-bg-hover);
  color: var(--text-primary);
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.spinning {
  animation: spin 0.9s linear infinite;
}

.filter-btn {
  width: auto;
  padding: 0 9px;
  font-size: 12px;
  display: flex;
  align-items: center;
  gap: 4px;
}

.unread-filter-btn.active {
  color: var(--primary-light);
  background: rgba(0, 184, 148, 0.12);
}

.star-filter-btn.active {
  color: #f39c12;
  background: rgba(243, 156, 18, 0.14);
}

.select-mode-btn.active {
  color: var(--primary-light);
  background: rgba(0, 184, 148, 0.14);
}

/* 批量操作工具栏 */
.batch-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: rgba(0, 184, 148, 0.08);
  border-bottom: 1px solid var(--glass-border);
  gap: 8px;
  flex-shrink: 0;
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  min-height: 40px;
}

.batch-left {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.batch-checkbox-btn {
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 2px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.batch-count-badge {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  white-space: nowrap;
  padding: 2px 7px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid var(--glass-border);
}

.batch-actions {
  display: flex;
  align-items: center;
  gap: 3px;
  flex-shrink: 0;
}

.batch-action-btn {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm, 6px);
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid var(--glass-border);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.batch-action-btn:hover:not(:disabled) {
  background: var(--glass-bg-hover);
  color: var(--text-primary);
  border-color: var(--glass-border-light);
  transform: translateY(-1px);
}

.batch-action-btn.star-btn:hover:not(:disabled) {
  color: #f39c12;
  background: rgba(243, 156, 18, 0.14);
  border-color: rgba(243, 156, 18, 0.25);
}

.batch-action-btn.danger:hover:not(:disabled) {
  background: rgba(255, 107, 107, 0.16);
  color: #ff6b6b;
  border-color: rgba(255, 107, 107, 0.3);
}

.batch-action-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
  pointer-events: none;
}

.batch-divider {
  width: 1px;
  height: 16px;
  background: var(--glass-border);
  margin: 0 2px;
  flex-shrink: 0;
}

.batch-close-btn {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm, 6px);
  background: transparent;
  border: 1px solid transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.batch-close-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-primary);
  border-color: var(--glass-border);
}

/* 批量栏动画 */
.batch-bar-anim-enter-active,
.batch-bar-anim-leave-active {
  transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  overflow: hidden;
}

.batch-bar-anim-enter-from,
.batch-bar-anim-leave-to {
  opacity: 0;
  max-height: 0;
  padding-top: 0;
  padding-bottom: 0;
  transform: translateY(-8px);
}

.batch-bar-anim-enter-to,
.batch-bar-anim-leave-from {
  opacity: 1;
  max-height: 48px;
  transform: translateY(0);
}

/* 列表主体与空状态 */
.maillist-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0;
  position: relative;
}

.maillist-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 220px;
  gap: 12px;
  color: var(--text-tertiary);
  font-size: 13px;
  padding: 40px 16px;
  text-align: center;
  user-select: none;
}

.loading-spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--glass-border);
  border-top-color: var(--primary-light);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

/* 复选框样式 */
.custom-checkbox {
  width: 15px;
  height: 15px;
  border-radius: 4px;
  border: 1.5px solid var(--text-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.05);
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.custom-checkbox.checked {
  background: var(--primary);
  border-color: var(--primary);
  color: #ffffff;
}

.custom-checkbox.partial {
  background: var(--primary);
  border-color: var(--primary);
}

.partial-dash {
  width: 7px;
  height: 2px;
  background: #ffffff;
  border-radius: 1px;
}

/* 列表项复选框容器 */
.mail-checkbox-wrap {
  width: 0;
  opacity: 0;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-top: 1px;
  transition: width 0.18s ease, opacity 0.18s ease, margin-right 0.18s ease;
  margin-right: 0;
  flex-shrink: 0;
}

.mail-item:hover .mail-checkbox-wrap,
.mail-checkbox-wrap.visible {
  width: 18px;
  opacity: 1;
  margin-right: 10px;
}

.mail-item {
  display: flex;
  align-items: flex-start;
  padding: 12px 14px 11px;
  cursor: pointer;
  transition: background 0.18s ease, border-color 0.18s ease;
  position: relative;
  border-bottom: 1px solid var(--glass-border);
}

.mail-item.checked {
  background: rgba(0, 184, 148, 0.14) !important;
}

.mail-item:hover {
  background: var(--glass-bg-hover);
}

.mail-item.selected {
  background: rgba(0, 184, 148, 0.10);
}

.mail-item::before {
  content: '';
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 2px;
  background: var(--primary-light);
  border-radius: 0 3px 3px 0;
  opacity: 0;
  transform: scaleY(0.7);
  transition: opacity 0.18s ease, transform 0.24s cubic-bezier(0.22, 0.61, 0.36, 1);
}

.mail-item.selected::before {
  opacity: 1;
  transform: scaleY(1);
}

.mail-info {
  flex: 1;
  min-width: 0;
}

.mail-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  min-height: 20px;
  gap: 8px;
}

.mail-sender-line {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  min-width: 0;
}

.mail-sender {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.mail-top-right {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

/* 星标按钮 */
.mail-star-btn {
  background: transparent;
  border: none;
  cursor: pointer;
  padding: 2px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-tertiary);
  opacity: 0;
  transition: opacity 0.15s ease, transform 0.15s ease, color 0.15s ease;
}

.mail-item:hover .mail-star-btn,
.mail-star-btn.active {
  opacity: 1;
}

.mail-star-btn.active {
  color: #f39c12;
}

.mail-star-btn:hover {
  transform: scale(1.18);
  color: #f39c12;
}

.mail-time {
  font-size: 11px;
  color: var(--text-tertiary);
  flex-shrink: 0;
  white-space: nowrap;
}

.mail-subject {
  font-size: 12px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 4px;
  line-height: 1.4;
}

.font-bold {
  font-weight: 600;
}

.unread-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--primary);
  flex-shrink: 0;
}

.mail-meta {
  margin-top: 6px;
  min-height: 15px;
}

.acc-label {
  font-size: 10px;
  color: var(--text-tertiary);
  display: inline-block;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--glass-bg);
}

:root[data-theme="light"] .acc-label {
  color: var(--primary);
}

/* 底部状态与分页栏 */
.mail-list-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 14px;
  border-top: 1px solid var(--glass-border);
  font-size: 11px;
  color: var(--text-tertiary);
  background: var(--bg-maillist);
  flex-shrink: 0;
  user-select: none;
}

.page-controls {
  display: flex;
  align-items: center;
  gap: 4px;
}

.page-controls span {
  font-size: 11px;
  color: var(--text-secondary);
  min-width: 36px;
  text-align: center;
}

.page-controls .toolbar-btn {
  width: 22px;
  height: 22px;
  font-size: 13px;
  line-height: 1;
  border-radius: var(--radius-sm);
}
</style>

<style>
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

.mail-items-container {
  position: relative;
  width: 100%;
}

.ctx-header {
  padding: 6px 12px 4px;
  font-size: 11px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.45);
  letter-spacing: 0.5px;
}

:root[data-theme="light"] .ctx-header {
  color: rgba(0, 0, 0, 0.45);
}

/* 彻底删除二次确认弹窗样式 */
.confirm-modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 10001;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.confirm-modal {
  width: 100%;
  max-width: 360px;
  background: rgba(30, 30, 48, 0.95);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 16px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6);
  padding: 24px;
  text-align: center;
  animation: modal-pop 0.22s cubic-bezier(0.16, 1, 0.3, 1);
}

:root[data-theme="light"] .confirm-modal {
  background: rgba(255, 255, 255, 0.98);
  border-color: rgba(0, 0, 0, 0.08);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.15);
}

@keyframes modal-pop {
  from {
    opacity: 0;
    transform: scale(0.92) translateY(8px);
  }
  to {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
}

.confirm-icon {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 14px;
}

.confirm-icon.danger {
  background: rgba(255, 107, 107, 0.15);
  color: #ff6b6b;
}

.confirm-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 8px;
  color: var(--text-primary, #ffffff);
}

.confirm-desc {
  font-size: 13px;
  color: var(--text-secondary, rgba(255, 255, 255, 0.7));
  margin: 0 0 20px;
  line-height: 1.5;
}

.confirm-buttons {
  display: flex;
  gap: 10px;
  justify-content: center;
}

.confirm-btn {
  flex: 1;
  padding: 9px 16px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  border: 1px solid transparent;
}

.confirm-btn.cancel {
  background: rgba(255, 255, 255, 0.08);
  color: var(--text-primary, #ffffff);
  border-color: rgba(255, 255, 255, 0.1);
}

:root[data-theme="light"] .confirm-btn.cancel {
  background: rgba(0, 0, 0, 0.06);
  color: rgba(0, 0, 0, 0.8);
  border-color: rgba(0, 0, 0, 0.1);
}

.confirm-btn.cancel:hover {
  background: rgba(255, 255, 255, 0.14);
}

.confirm-btn.danger {
  background: #ff6b6b;
  color: #ffffff;
}

.confirm-btn.danger:hover {
  background: #fa5252;
}
</style>
