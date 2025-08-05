import { useState, useEffect } from 'react'
import { Provider } from '../shared/types'
import ProviderList from './components/ProviderList'
import AddProviderModal from './components/AddProviderModal'
import EditProviderModal from './components/EditProviderModal'
import './App.css'

function App() {
  const [providers, setProviders] = useState<Record<string, Provider>>({})
  const [currentProviderId, setCurrentProviderId] = useState<string>('')
  const [isAddModalOpen, setIsAddModalOpen] = useState(false)
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


  // 生成唯一ID
  const generateId = () => {
    return Date.now().toString(36) + Math.random().toString(36).substr(2, 9)
  }

  const handleAddProvider = async (provider: Omit<Provider, 'id'>) => {
    const newProvider: Provider = {
      ...provider,
      id: generateId()
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
          onSwitch={handleSwitchProvider}
          onDelete={handleDeleteProvider}
          onEdit={setEditingProviderId}
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