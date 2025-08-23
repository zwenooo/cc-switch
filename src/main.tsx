import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'
// 导入 Tauri API（自动绑定到 window.electronAPI）
import './lib/tauri-api'

// 根据平台添加 body class，便于平台特定样式
if (window.platform?.isMac) {
  document.body.classList.add('is-mac')
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
