import { useState, useEffect } from 'react'
import { Provider, ProviderStatus } from '../shared/types'
import ProviderList from './components/ProviderList'
import AddProviderModal from './components/AddProviderModal'
import EditProviderModal from './components/EditProviderModal'
import './App.css'

function App() {
  const [providers, setProviders] = useState<Record<string, Provider>>({})
  const [currentProviderId, setCurrentProviderId] = useState<string>('')
  const [statuses, setStatuses] = useState<Record<string, ProviderStatus>>({})
  const [isAddModalOpen, setIsAddModalOpen] = useState(false)
  const [checkingStatus, setCheckingStatus] = useState<Record<string, boolean>>({})
  const [configPath, setConfigPath] = useState<string>('')
  const [editingProviderId, setEditingProviderId] = useState<string | null>(null)

  // 加载供应商列表
  useEffect(() => {
    loadProviders()
    loadConfigPath()
  }, [])


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
    // 功能开发中
    alert('状态检查功能开发中')
  }

  const checkSingleStatus = async (providerId: string) => {
    const provider = providers[providerId]
    if (!provider) return

    setCheckingStatus(prev => ({ ...prev, [providerId]: true }))
    
    try {
      // 暂时显示开发中状态
      const status: ProviderStatus = {
        isOnline: false,
        responseTime: -1,
        lastChecked: new Date(),
        error: '功能开发中'
      }
      setStatuses(prev => ({ ...prev, [providerId]: status }))
    } catch (error) {
      console.error('检查状态失败:', error)
    } finally {
      setCheckingStatus(prev => ({ ...prev, [providerId]: false }))
    }
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

  const handleEditProvider = async (provider: Provider) => {
    try {
      await window.electronAPI.updateProvider(provider)
      await loadProviders()
      setEditingProviderId(null)
      alert('保存成功！')
    } catch (error) {
      console.error('更新供应商失败:', error)
      alert('保存失败，请重试')
    }
  }

  const handleSelectConfigFile = async () => {
    const selectedPath = await window.electronAPI.selectConfigFile()
    if (selectedPath) {
      setConfigPath(selectedPath)
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
          >
            检查状态（开发中）
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
          checkingStatus={checkingStatus}
          onSwitch={handleSwitchProvider}
          onDelete={handleDeleteProvider}
          onEdit={setEditingProviderId}
          onCheckStatus={checkSingleStatus}
        />
        
        {configPath && (
          <div className="config-path">
            <span>配置文件位置: {configPath}</span>
            <button 
              className="browse-btn" 
              onClick={handleSelectConfigFile}
              title="浏览选择配置文件"
            >
              浏览
            </button>
          </div>
        )}
      </main>

      {isAddModalOpen && (
        <AddProviderModal
          onAdd={handleAddProvider}
          onClose={() => setIsAddModalOpen(false)}
        />
      )}

      {editingProviderId && providers[editingProviderId] && (
        <EditProviderModal
          provider={providers[editingProviderId]}
          onSave={handleEditProvider}
          onClose={() => setEditingProviderId(null)}
        />
      )}
    </div>
  )
}

export default App