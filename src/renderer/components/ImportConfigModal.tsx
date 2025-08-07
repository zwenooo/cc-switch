import React, { useState } from 'react'
import './AddProviderModal.css'

interface ImportConfigModalProps {
  onImport: (name: string) => void
  onClose: () => void
  isEmpty?: boolean  // 供应商列表是否为空
}

const ImportConfigModal: React.FC<ImportConfigModalProps> = ({ onImport, onClose, isEmpty = false }) => {
  const [name, setName] = useState('')
  const [error, setError] = useState('')

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    
    if (!name.trim()) {
      setError('请输入供应商名称')
      return
    }
    
    onImport(name.trim())
  }

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <h2>{isEmpty ? '供应商列表为空' : '导入当前配置'}</h2>
        
        {isEmpty ? (
          <p style={{marginBottom: '1.5rem', color: '#666', fontSize: '0.9rem'}}>
            当前还没有任何供应商配置。您可以将当前的 Claude Code 配置 
            <code>~/.claude/settings.json</code> 导入为一个供应商配置。
            <br />
            <strong>注意：</strong>这不会修改您当前的配置文件。
          </p>
        ) : (
          <p style={{marginBottom: '1.5rem', color: '#666', fontSize: '0.9rem'}}>
            将当前的 <code>~/.claude/settings.json</code> 配置文件导入为一个新的供应商。
            <br />
            <strong>注意：</strong>这不会修改您当前的配置文件。
          </p>
        )}

        {error && <div className="error-message">{error}</div>}

        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="name">供应商名称 *</label>
            <input
              type="text"
              id="name"
              name="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={isEmpty ? "例如：我的默认配置" : "例如：我的当前配置"}
              required
              autoFocus
            />
          </div>

          <div className="form-actions">
            <button type="button" className="cancel-btn" onClick={onClose}>
              {isEmpty ? '稍后设置' : '取消'}
            </button>
            <button type="submit" className="submit-btn">
              导入
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

export default ImportConfigModal