import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'
// 导入 Tauri API（自动绑定到 window.api）
import './lib/tauri-api'
import { platform as osPlatform } from '@tauri-apps/api/os'

// 根据平台添加 body class，便于平台特定样式
osPlatform().then((p) => {
  if (p === 'darwin') {
    document.body.classList.add('is-mac')
  }
}).catch(() => {
  // 忽略平台检测失败
})

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
