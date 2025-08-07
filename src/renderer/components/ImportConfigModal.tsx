import React, { useState } from 'react'
import './AddProviderModal.css'

interface ImportConfigModalProps {
  onImport: (name: string) => void
  onClose: () => void
}

const ImportConfigModal: React.FC<ImportConfigModalProps> = ({ onImport, onClose }) => {
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
        <h2>导入当前配置</h2>
        
        <p style={{marginBottom: '1.5rem', color: '#666', fontSize: '0.9rem'}}>
          将当前的 <code>~/.claude/settings.json</code> 配置文件导入为一个新的供应商。
          <br />
          <strong>注意：</strong>这不会修改您当前的配置文件。
        </p>

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
              placeholder="例如：我的当前配置"
              required
              autoFocus
            />
          </div>

          <div className="form-actions">
            <button type="button" className="cancel-btn" onClick={onClose}>
              取消
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