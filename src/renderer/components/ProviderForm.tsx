import React, { useState, useEffect } from 'react'
import { Provider } from '../../shared/types'
import { updateCoAuthoredSetting, checkCoAuthoredSetting, extractWebsiteUrl } from '../utils/providerConfigUtils'
import { providerPresets } from '../config/providerPresets'
import './AddProviderModal.css'

interface ProviderFormProps {
  title: string
  submitText: string
  initialData?: Provider
  showPresets?: boolean
  onSubmit: (data: Omit<Provider, 'id'>) => void
  onClose: () => void
}

const ProviderForm: React.FC<ProviderFormProps> = ({
  title,
  submitText,
  initialData,
  showPresets = false,
  onSubmit,
  onClose
}) => {
  const [formData, setFormData] = useState({
    name: initialData?.name || '',
    websiteUrl: initialData?.websiteUrl || '',
    settingsConfig: initialData ? JSON.stringify(initialData.settingsConfig, null, 2) : ''
  })
  const [error, setError] = useState('')
  const [disableCoAuthored, setDisableCoAuthored] = useState(false)
  const [selectedPreset, setSelectedPreset] = useState<number | null>(null)

  // 初始化时检查禁用签名状态
  useEffect(() => {
    if (initialData) {
      const configString = JSON.stringify(initialData.settingsConfig, null, 2)
      const hasCoAuthoredDisabled = checkCoAuthoredSetting(configString)
      setDisableCoAuthored(hasCoAuthoredDisabled)
    }
  }, [initialData])

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

    let settingsConfig: Record<string, any>
    
    try {
      settingsConfig = JSON.parse(formData.settingsConfig)
    } catch (err) {
      setError('配置JSON格式错误，请检查语法')
      return
    }

    onSubmit({
      name: formData.name,
      websiteUrl: formData.websiteUrl,
      settingsConfig
    })
  }

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
  ) => {
    const { name, value } = e.target
    
    if (name === 'settingsConfig') {
      // 当用户修改配置时，尝试自动提取官网地址
      const extractedWebsiteUrl = extractWebsiteUrl(value)
      
      // 同时检查并同步选择框状态
      const hasCoAuthoredDisabled = checkCoAuthoredSetting(value)
      setDisableCoAuthored(hasCoAuthoredDisabled)
      
      setFormData({
        ...formData,
        [name]: value,
        // 只有在官网地址为空时才自动填入
        websiteUrl: formData.websiteUrl || extractedWebsiteUrl,
      })
    } else {
      setFormData({
        ...formData,
        [name]: value,
      })
    }
  }

  // 处理选择框变化
  const handleCoAuthoredToggle = (checked: boolean) => {
    setDisableCoAuthored(checked)
    
    // 更新JSON配置
    const updatedConfig = updateCoAuthoredSetting(formData.settingsConfig, checked)
    setFormData({
      ...formData,
      settingsConfig: updatedConfig,
    })
  }

  const applyPreset = (preset: typeof providerPresets[0], index: number) => {
    const configString = JSON.stringify(preset.settingsConfig, null, 2)
    
    setFormData({
      name: preset.name,
      websiteUrl: preset.websiteUrl,
      settingsConfig: configString
    })
    
    // 设置选中的预设
    setSelectedPreset(index)
    
    // 同步选择框状态
    const hasCoAuthoredDisabled = checkCoAuthoredSetting(configString)
    setDisableCoAuthored(hasCoAuthoredDisabled)
  }

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <h2>{title}</h2>

        {error && <div className="error-message">{error}</div>}

        {showPresets && (
          <div className="presets">
            <label>快速选择模板：</label>
            <div className="preset-buttons">
              {providerPresets.map((preset, index) => (
                <button
                  key={index}
                  type="button"
                  className={`preset-btn ${selectedPreset === index ? 'selected' : ''}`}
                  onClick={() => applyPreset(preset, index)}
                >
                  {preset.name}
                </button>
              ))}
            </div>
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
            <div className="label-with-checkbox">
              <label htmlFor="settingsConfig">Claude Code 配置 (JSON) *</label>
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={disableCoAuthored}
                  onChange={(e) => handleCoAuthoredToggle(e.target.checked)}
                />
                禁止 Claude Code 签名
              </label>
            </div>
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
              {submitText}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

export default ProviderForm