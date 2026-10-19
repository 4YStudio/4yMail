import { createApp } from 'vue'
import App from './App.vue'
import './assets/styles/global.css'
// Initialize the compatibility API before mounting components that use it.
import '../tauri-bridge.js'

createApp(App).mount('#app')
