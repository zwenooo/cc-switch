import { app, BrowserWindow, ipcMain, dialog } from 'electron'
import path from 'path'
import Store from 'electron-store'
import { Provider, AppConfig } from '../shared/types'
import { checkProviderStatus, switchProvider, getClaudeCodeConfig } from './services'

const store = new Store<AppConfig>()

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
  return store.get('providers', {})
})

ipcMain.handle('getCurrentProvider', () => {
  return store.get('current', '')
})

ipcMain.handle('addProvider', (_, provider: Provider) => {
  const providers = store.get('providers', {})
  providers[provider.id] = provider
  store.set('providers', providers)
  return true
})

ipcMain.handle('deleteProvider', (_, id: string) => {
  const providers = store.get('providers', {})
  delete providers[id]
  store.set('providers', providers)
  return true
})

ipcMain.handle('updateProvider', (_, provider: Provider) => {
  const providers = store.get('providers', {})
  providers[provider.id] = provider
  store.set('providers', providers)
  return true
})

ipcMain.handle('checkStatus', async (_, provider: Provider) => {
  return await checkProviderStatus(provider)
})

ipcMain.handle('switchProvider', async (_, providerId: string) => {
  const providers = store.get('providers', {})
  const provider = providers[providerId]
  if (provider) {
    const success = await switchProvider(provider)
    if (success) {
      store.set('current', providerId)
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