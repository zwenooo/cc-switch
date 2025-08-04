import React, { useState, useEffect } from 'react'
import { Provider, ProviderStatus } from '../shared/types'
import ProviderList from './components/ProviderList'
import AddProviderModal from './components/AddProviderModal'
import './App.css'

function App() {
  const [providers, setProviders] = useState<Record<string, Provider>>({})
  const [currentProviderId, setCurrentProviderId] = useState<string>('')
  const [statuses, setStatuses] = useState<Record<string, ProviderStatus>>({})
  const [isAddModalOpen, setIsAddModalOpen] = useState(false)
  const [isRefreshing, setIsRefreshing] = useState(false)
  const [configPath, setConfigPath] = useState<string>('')

  // 加载供应商列表
  useEffect(() => {
    loadProviders()
    loadConfigPath()
  }, [])

  // 定时检查状态
  useEffect(() => {
    checkAllStatuses()
    const interval = setInterval(checkAllStatuses, 30000) // 每30秒检查一次
    return () => clearInterval(interval)
  }, [providers])

  const loadProviders = async () => {
    const loadedProviders = await window.electronAPI.getProviders()
    const currentId = await window.electronAPI.getCurrentProvider()
    setProviders(loadedProviders)
    setCurrentProviderId(currentId)
  }

  const loadConfigPath = async () => {
    const path = await window.electronAPI.getClaudeCodeConfigPath()
    setConfigPath(path)
  }

  const checkAllStatuses = async () => {
    if (Object.keys(providers).length === 0) return
    
    setIsRefreshing(true)
    const newStatuses: Record<string, ProviderStatus> = {}
    
    await Promise.all(
      Object.values(providers).map(async (provider) => {
        const status = await window.electronAPI.checkStatus(provider)
        newStatuses[provider.id] = status
      })
    )
    
    setStatuses(newStatuses)
    setIsRefreshing(false)
  }

  const handleAddProvider = async (provider: Omit<Provider, 'id'>) => {
    const newProvider: Provider = {
      ...provider,
      id: Date.now().toString()
    }
    await window.electronAPI.addProvider(newProvider)
    await loadProviders()
    setIsAddModalOpen(false)
  }

  const handleDeleteProvider = async (id: string) => {
    if (confirm('确定要删除这个供应商吗？')) {
      await window.electronAPI.deleteProvider(id)
      await loadProviders()
    }
  }

  const handleSwitchProvider = async (id: string) => {
    const success = await window.electronAPI.switchProvider(id)
    if (success) {
      setCurrentProviderId(id)
      alert('切换成功！')
    } else {
      alert('切换失败，请检查配置')
    }
  }

  return (
    <div className="app">
      <header className="app-header">
        <h1>Claude Code 供应商切换器</h1>
        <div className="header-actions">
          <button 
            className="refresh-btn" 
            onClick={checkAllStatuses} 
            disabled={isRefreshing}
          >
            {isRefreshing ? '检查中...' : '刷新状态'}
          </button>
          <button 
            className="add-btn" 
            onClick={() => setIsAddModalOpen(true)}
          >
            添加供应商
          </button>
        </div>
      </header>

      <main className="app-main">
        <ProviderList
          providers={providers}
          currentProviderId={currentProviderId}
          statuses={statuses}
          onSwitch={handleSwitchProvider}
          onDelete={handleDeleteProvider}
        />
        
        {configPath && (
          <div className="config-path">
            配置文件位置: {configPath}
          </div>
        )}
      </main>

      {isAddModalOpen && (
        <AddProviderModal
          onAdd={handleAddProvider}
          onClose={() => setIsAddModalOpen(false)}
        />
      )}
    </div>
  )
}

export default App