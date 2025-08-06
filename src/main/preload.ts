import { contextBridge, ipcRenderer } from 'electron'
import { Provider } from '../shared/types'

contextBridge.exposeInMainWorld('electronAPI', {
  getProviders: () => ipcRenderer.invoke('getProviders'),
  getCurrentProvider: () => ipcRenderer.invoke('getCurrentProvider'),
  addProvider: (provider: Provider) => ipcRenderer.invoke('addProvider', provider),
  deleteProvider: (id: string) => ipcRenderer.invoke('deleteProvider', id),
  updateProvider: (provider: Provider) => ipcRenderer.invoke('updateProvider', provider),
  checkStatus: (provider: Provider) => ipcRenderer.invoke('checkStatus', provider),
  switchProvider: (providerId: string) => ipcRenderer.invoke('switchProvider', providerId),
  getClaudeCodeConfigPath: () => ipcRenderer.invoke('getClaudeCodeConfigPath'),
  selectConfigFile: () => ipcRenderer.invoke('selectConfigFile'),
  openExternal: (url: string) => ipcRenderer.invoke('openExternal', url)
})