import React, { useState } from 'react'
import { Provider } from '../../shared/types'
import './AddProviderModal.css'

interface AddProviderModalProps {
  onAdd: (provider: Omit<Provider, 'id'>) => void
  onClose: () => void
  onError?: (message: string) => void
}

const AddProviderModal: React.FC<AddProviderModalProps> = ({ onAdd, onClose, onError }) => {
  const [formData, setFormData] = useState({
    name: '',
    apiUrl: '',
    apiKey: ''
  })
  const [showPassword, setShowPassword] = useState(false)
  const [error, setError] = useState<string>('')

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    
    if (!formData.name || !formData.apiUrl || !formData.apiKey) {
      setError('请填写所有必填字段')
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
      name: 'YesCode',
      apiUrl: 'https://co.yes.vg'
    },
    {
      name: 'PackyCode',
      apiUrl: 'https://api.packycode.com'
    }
  ]

  const applyPreset = (preset: typeof presets[0]) => {
    setFormData({
      ...formData,
      name: preset.name,
      apiUrl: preset.apiUrl
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
          {error && (
            <div style={{ color: 'red', marginBottom: '1rem', padding: '0.5rem', backgroundColor: '#ffe6e6', borderRadius: '4px' }}>
              {error}
            </div>
          )}
          
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
            <div className="password-input-wrapper">
              <input
                type={showPassword ? "text" : "password"}
                id="apiKey"
                name="apiKey"
                value={formData.apiKey}
                onChange={handleChange}
                placeholder={formData.name === 'YesCode' ? 'cr_...' : 'sk-...'}
                required
              />
              <button
                type="button"
                className="password-toggle"
                onClick={() => setShowPassword(!showPassword)}
                tabIndex={-1}
                title={showPassword ? "隐藏密码" : "显示密码"}
              >
                {showPassword ? (
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                    <circle cx="12" cy="12" r="3" />
                  </svg>
                ) : (
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                    <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" />
                    <line x1="1" y1="1" x2="23" y2="23" />
                  </svg>
                )}
              </button>
            </div>
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