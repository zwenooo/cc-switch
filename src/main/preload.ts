import { contextBridge, ipcRenderer } from 'electron'
import { Provider } from '../shared/types'

contextBridge.exposeInMainWorld('electronAPI', {
  getProviders: () => ipcRenderer.invoke('getProviders'),
  getCurrentProvider: () => ipcRenderer.invoke('getCurrentProvider'),
  addProvider: (provider: Provider) => ipcRenderer.invoke('addProvider', provider),
  deleteProvider: (id: string) => ipcRenderer.invoke('deleteProvider', id),
  updateProvider: (provider: Provider) => ipcRenderer.invoke('updateProvider', provider),
  switchProvider: (providerId: string) => ipcRenderer.invoke('switchProvider', providerId),
  importCurrentConfigAsDefault: () => ipcRenderer.invoke('importCurrentConfigAsDefault'),
  getClaudeCodeConfigPath: () => ipcRenderer.invoke('getClaudeCodeConfigPath'),
  selectConfigFile: () => ipcRenderer.invoke('selectConfigFile'),
  openConfigFolder: () => ipcRenderer.invoke('openConfigFolder'),
  openExternal: (url: string) => ipcRenderer.invoke('openExternal', url)
})

// 暴露平台信息给渲染进程，用于平台特定样式控制
contextBridge.exposeInMainWorld('platform', {
  isMac: process.platform === 'darwin'
})
