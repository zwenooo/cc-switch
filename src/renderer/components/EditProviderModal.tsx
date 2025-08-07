import React, { useState, useEffect } from 'react'
import { Provider } from '../../shared/types'
import './AddProviderModal.css'

interface EditProviderModalProps {
  provider: Provider
  onSave: (provider: Provider) => void
  onClose: () => void
}

const EditProviderModal: React.FC<EditProviderModalProps> = ({ provider, onSave, onClose }) => {
  const [formData, setFormData] = useState({
    name: provider.name,
    websiteUrl: provider.websiteUrl || '',
    settingsConfig: JSON.stringify(provider.settingsConfig, null, 2)
  })
  const [error, setError] = useState('')

  // 初始化时更新表单数据
  useEffect(() => {
    setFormData({
      name: provider.name,
      websiteUrl: provider.websiteUrl || '',
      settingsConfig: JSON.stringify(provider.settingsConfig, null, 2)
    })
  }, [provider])

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    
    if (!formData.name) {
      setError('请填写供应商名称')
      return
    }

    if (!formData.settingsConfig.trim()) {
      setError('请填写配置内容')
      return
    }

    let settingsConfig: object
    
    try {
      settingsConfig = JSON.parse(formData.settingsConfig)
    } catch (err) {
      setError('配置JSON格式错误，请检查语法')
      return
    }

    onSave({
      ...provider,
      name: formData.name,
      websiteUrl: formData.websiteUrl,
      settingsConfig,
      updatedAt: Date.now()
    })
  }

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target
    setFormData({
      ...formData,
      [name]: value
    })
  }

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <h2>编辑供应商</h2>

        {error && (
          <div className="error-message">
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="name">供应商名称 *</label>
            <input
              type="text"
              id="name"
              name="name"
              value={formData.name}
              onChange={handleChange}
              placeholder="例如：Anthropic 官方"
              required
              autoComplete="off"
            />
          </div>

          <div className="form-group">
            <label htmlFor="websiteUrl">官网地址</label>
            <input
              type="url"
              id="websiteUrl"
              name="websiteUrl"
              value={formData.websiteUrl}
              onChange={handleChange}
              placeholder="https://example.com（可选）"
              autoComplete="off"
            />
          </div>

          <div className="form-group">
            <label htmlFor="settingsConfig">Claude Code 配置 (JSON) *</label>
            <textarea
              id="settingsConfig"
              name="settingsConfig"
              value={formData.settingsConfig}
              onChange={handleChange}
              placeholder={`{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
    "ANTHROPIC_AUTH_TOKEN": "sk-your-api-key-here"
  }
}`}
              rows={12}
              style={{fontFamily: 'monospace', fontSize: '14px'}}
              required
            />
            <small className="field-hint">
              完整的 Claude Code settings.json 配置内容
            </small>
          </div>

          <div className="form-actions">
            <button type="button" className="cancel-btn" onClick={onClose}>
              取消
            </button>
            <button type="submit" className="submit-btn">
              保存
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

export default EditProviderModal