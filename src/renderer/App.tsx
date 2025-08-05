import { useState, useEffect } from 'react'
import { Provider } from '../shared/types'
import ProviderList from './components/ProviderList'
import AddProviderModal from './components/AddProviderModal'
import EditProviderModal from './components/EditProviderModal'
import ConfirmModal from './components/ConfirmModal'
import MessageModal from './components/MessageModal'
import './App.css'

function App() {
  const [providers, setProviders] = useState<Record<string, Provider>>({})
  const [currentProviderId, setCurrentProviderId] = useState<string>('')
  const [isAddModalOpen, setIsAddModalOpen] = useState(false)
  const [configPath, setConfigPath] = useState<string>('')
  const [editingProviderId, setEditingProviderId] = useState<string | null>(null)
  
  // Modal states
  const [confirmModal, setConfirmModal] = useState<{
    show: boolean
    title: string
    message: string
    onConfirm: () => void
  } | null>(null)
  const [messageModal, setMessageModal] = useState<{
    show: boolean
    title: string
    message: string
    type: 'success' | 'error' | 'info'
  } | null>(null)

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
    setConfirmModal({
      show: true,
      title: '删除供应商',
      message: '确定要删除这个供应商吗？',
      onConfirm: async () => {
        await window.electronAPI.deleteProvider(id)
        await loadProviders()
        setConfirmModal(null)
      }
    })
  }

  const handleSwitchProvider = async (id: string) => {
    const provider = providers[id]
    if (!provider) return
    
    // 如果是当前供应商，直接返回
    if (id === currentProviderId) return
    
    setConfirmModal({
      show: true,
      title: '切换供应商',
      message: `确定要切换到"${provider.name}"吗？`,
      onConfirm: async () => {
        const success = await window.electronAPI.switchProvider(id)
        if (success) {
          setCurrentProviderId(id)
          setMessageModal({
            show: true,
            title: '切换成功',
            message: '供应商已成功切换！',
            type: 'success'
          })
        } else {
          setMessageModal({
            show: true,
            title: '切换失败',
            message: '切换失败，请检查配置',
            type: 'error'
          })
        }
        setConfirmModal(null)
      }
    })
  }

  const handleEditProvider = async (provider: Provider) => {
    try {
      await window.electronAPI.updateProvider(provider)
      await loadProviders()
      setEditingProviderId(null)
      setMessageModal({
        show: true,
        title: '保存成功',
        message: '供应商信息已更新！',
        type: 'success'
      })
    } catch (error) {
      console.error('更新供应商失败:', error)
      setMessageModal({
        show: true,
        title: '保存失败',
        message: '保存失败，请重试',
        type: 'error'
      })
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

      {confirmModal && (
        <ConfirmModal
          title={confirmModal.title}
          message={confirmModal.message}
          onConfirm={confirmModal.onConfirm}
          onCancel={() => setConfirmModal(null)}
        />
      )}

      {messageModal && (
        <MessageModal
          title={messageModal.title}
          message={messageModal.message}
          type={messageModal.type}
          onClose={() => setMessageModal(null)}
        />
      )}
    </div>
  )
}

export default App