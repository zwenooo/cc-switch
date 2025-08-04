import React, { useState } from 'react'
import { Provider } from '../../shared/types'
import './AddProviderModal.css'

interface AddProviderModalProps {
  onAdd: (provider: Omit<Provider, 'id'>) => void
  onClose: () => void
}

const AddProviderModal: React.FC<AddProviderModalProps> = ({ onAdd, onClose }) => {
  const [formData, setFormData] = useState({
    name: '',
    apiUrl: '',
    apiKey: '',
    model: 'claude-3-opus-20240229'
  })

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    
    if (!formData.name || !formData.apiUrl || !formData.apiKey) {
      alert('请填写所有必填字段')
      return
    }

    onAdd(formData)
  }

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    setFormData({
      ...formData,
      [e.target.name]: e.target.value
    })
  }

  // 预设的供应商配置
  const presets = [
    {
      name: '官方 Anthropic',
      apiUrl: 'https://api.anthropic.com',
      model: 'claude-3-opus-20240229'
    },
    {
      name: 'OpenRouter',
      apiUrl: 'https://openrouter.ai/api/v1',
      model: 'anthropic/claude-3-opus'
    }
  ]

  const applyPreset = (preset: typeof presets[0]) => {
    setFormData({
      ...formData,
      name: preset.name,
      apiUrl: preset.apiUrl,
      model: preset.model
    })
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <h2>添加新供应商</h2>
        
        <div className="presets">
          <label>快速选择：</label>
          <div className="preset-buttons">
            {presets.map((preset, index) => (
              <button
                key={index}
                type="button"
                className="preset-btn"
                onClick={() => applyPreset(preset)}
              >
                {preset.name}
              </button>
            ))}
          </div>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="name">供应商名称 *</label>
            <input
              type="text"
              id="name"
              name="name"
              value={formData.name}
              onChange={handleChange}
              placeholder="例如：官方 Anthropic"
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="apiUrl">API 地址 *</label>
            <input
              type="url"
              id="apiUrl"
              name="apiUrl"
              value={formData.apiUrl}
              onChange={handleChange}
              placeholder="https://api.anthropic.com"
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="apiKey">API Key *</label>
            <input
              type="password"
              id="apiKey"
              name="apiKey"
              value={formData.apiKey}
              onChange={handleChange}
              placeholder="sk-ant-..."
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="model">模型名称</label>
            <input
              type="text"
              id="model"
              name="model"
              value={formData.model}
              onChange={handleChange}
              placeholder="claude-3-opus-20240229"
            />
          </div>

          <div className="form-actions">
            <button type="button" className="cancel-btn" onClick={onClose}>
              取消
            </button>
            <button type="submit" className="submit-btn">
              添加
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

export default AddProviderModal