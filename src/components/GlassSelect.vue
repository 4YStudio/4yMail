<template>
  <div class="glass-select-container" ref="containerRef" v-bind="$attrs">
    <div 
      class="glass-select-trigger" 
      :class="{ active: isOpen }"
      @click="toggle"
    >
      <span class="selected-text">{{ selectedLabel }}</span>
      <svg 
        class="arrow-icon" 
        :class="{ rotated: isOpen }"
        width="14" height="14" viewBox="0 0 24 24" fill="none"
      >
        <path d="M6 9l6 6 6-6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>

    <Transition name="select-pop">
      <div v-if="isOpen" class="glass-select-dropdown">
        <div 
          v-for="option in options" 
          :key="option.value"
          class="select-option"
          :class="{ selected: modelValue === option.value }"
          @click="selectOption(option.value)"
        >
          {{ option.label }}
          <svg v-if="modelValue === option.value" class="check-icon" width="14" height="14" viewBox="0 0 24 24" fill="none">
            <path d="M20 6L9 17l-5-5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'

const props = defineProps({
  modelValue: [String, Number, Boolean],
  options: {
    type: Array,
    required: true,
    // [{ label: 'text', value: 'val' }]
  },
  placeholder: {
    type: String,
    default: '请选择...'
  }
})

const emit = defineEmits(['update:modelValue'])

const isOpen = ref(false)
const containerRef = ref(null)

const selectedLabel = computed(() => {
  const option = props.options.find(opt => opt.value === props.modelValue)
  return option ? option.label : props.placeholder
})

function toggle() {
  isOpen.value = !isOpen.value
}

function selectOption(val) {
  emit('update:modelValue', val)
  isOpen.value = false
}

function handleClickOutside(e) {
  if (containerRef.value && !containerRef.value.contains(e.target)) {
    isOpen.value = false
  }
}

onMounted(() => {
  document.addEventListener('mousedown', handleClickOutside)
})

onUnmounted(() => {
  document.removeEventListener('mousedown', handleClickOutside)
})
</script>

<style scoped>
.glass-select-container {
  position: relative;
  width: 100%;
}

.glass-select-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--input-bg);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
  min-height: 36px;
}

.glass-select-trigger:hover {
  background: var(--glass-bg-hover);
  border-color: var(--glass-border-light);
}

.glass-select-trigger.active {
  border-color: var(--primary);
  background: var(--glass-bg-active);
  box-shadow: 0 0 0 2px rgba(0, 184, 148, 0.15);
}

.selected-text {
  font-size: 13px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.arrow-icon {
  color: var(--text-tertiary);
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  margin-left: 8px;
  flex-shrink: 0;
}

.arrow-icon.rotated {
  transform: rotate(180deg);
  color: var(--primary);
}

.glass-select-dropdown {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  right: 0;
  z-index: 100;
  background: rgba(30, 30, 50, 0.95);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border: 1px solid var(--glass-border-light);
  border-radius: var(--radius-md);
  padding: 6px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
  max-height: 240px;
  overflow-y: auto;
}

:root[data-theme="light"] .glass-select-dropdown {
  background: rgba(255, 255, 255, 0.95);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.08);
}

.select-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.12s ease;
}

.select-option:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--text-primary);
}

:root[data-theme="light"] .select-option:hover {
  background: rgba(0, 0, 0, 0.04);
}

.select-option.selected {
  background: rgba(0, 184, 148, 0.15);
  color: var(--primary-light);
  font-weight: 500;
}

:root[data-theme="light"] .select-option.selected {
  color: var(--primary);
}

.check-icon {
  color: var(--primary);
}

/* 动画 */
.select-pop-enter-active {
  transition: all 0.2s cubic-bezier(0.25, 1, 0.5, 1);
}

.select-pop-leave-active {
  transition: all 0.15s ease-in;
}

.select-pop-enter-from {
  opacity: 0;
  transform: translateY(-8px) scale(0.98);
}

.select-pop-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.98);
}

/* 隐藏 Webkit 的数字输入框箭头，以防万一 container 里有 input */
input::-webkit-outer-spin-button,
input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
</style>
