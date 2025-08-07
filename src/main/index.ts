import { app, BrowserWindow, ipcMain, dialog, shell } from 'electron'
import path from 'path'
import { Provider } from '../shared/types'
import { switchProvider, getClaudeCodeConfig } from './services'
import { store } from './store'

let mainWindow: BrowserWindow | null = null

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 800,
    height: 600,
    webPreferences: {
      preload: path.join(__dirname, '../main/preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    },
    titleBarStyle: 'hiddenInset',
    autoHideMenuBar: true
  })

  if (app.isPackaged) {
    mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'))
  } else {
    mainWindow.loadURL('http://localhost:3000')
    mainWindow.webContents.openDevTools()
  }

  mainWindow.on('closed', () => {
    mainWindow = null
  })
}

app.whenReady().then(() => {
  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow()
    }
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

// IPC handlers
ipcMain.handle('getProviders', () => {
  return store.get('providers', {} as Record<string, Provider>)
})

ipcMain.handle('getCurrentProvider', () => {
  return store.get('current', '')
})

ipcMain.handle('addProvider', async (_, provider: Provider) => {
  const providers = store.get('providers', {} as Record<string, Provider>)
  providers[provider.id] = provider
  await store.set('providers', providers)
  return true
})

ipcMain.handle('deleteProvider', async (_, id: string) => {
  const providers = store.get('providers', {} as Record<string, Provider>)
  delete providers[id]
  await store.set('providers', providers)
  return true
})

ipcMain.handle('updateProvider', async (_, provider: Provider) => {
  const providers = store.get('providers', {} as Record<string, Provider>)
  const currentProviderId = store.get('current', '')
  
  providers[provider.id] = provider
  await store.set('providers', providers)
  
  // 如果编辑的是当前激活的供应商，同时更新Claude Code配置
  if (provider.id === currentProviderId) {
    const success = await switchProvider(provider)
    if (!success) {
      console.error('更新当前供应商的Claude Code配置失败')
      return false
    }
  }
  
  return true
})

ipcMain.handle('switchProvider', async (_, providerId: string) => {
  const providers = store.get('providers', {} as Record<string, Provider>)
  const provider = providers[providerId]
  if (provider) {
    const success = await switchProvider(provider)
    if (success) {
      await store.set('current', providerId)
    }
    return success
  }
  return false
})

ipcMain.handle('getClaudeCodeConfigPath', () => {
  return getClaudeCodeConfig().path
})

ipcMain.handle('selectConfigFile', async () => {
  if (!mainWindow) return null
  
  const result = await dialog.showOpenDialog(mainWindow, {
    properties: ['openFile'],
    title: '选择 Claude Code 配置文件',
    filters: [
      { name: 'JSON 文件', extensions: ['json'] },
      { name: '所有文件', extensions: ['*'] }
    ],
    defaultPath: 'settings.json'
  })
  
  if (result.canceled || result.filePaths.length === 0) {
    return null
  }
  
  return result.filePaths[0]
})

ipcMain.handle('openExternal', async (_, url: string) => {
  try {
    await shell.openExternal(url)
    return true
  } catch (error) {
    console.error('打开外部链接失败:', error)
    return false
  }
})