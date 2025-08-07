import React, { useState } from "react";
import { Provider } from "../../shared/types";
import "./AddProviderModal.css";

interface AddProviderModalProps {
  onAdd: (provider: Omit<Provider, "id">) => void;
  onClose: () => void;
}

const AddProviderModal: React.FC<AddProviderModalProps> = ({
  onAdd,
  onClose,
}) => {
  const [formData, setFormData] = useState({
    name: "",
    websiteUrl: "",
    settingsConfig: ""
  });
  const [error, setError] = useState("");

  // 预设的供应商配置模板
  const presets = [
    {
      name: "Anthropic 官方",
      websiteUrl: "https://console.anthropic.com",
      settingsConfig: {
        "env": {
          "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
          "ANTHROPIC_AUTH_TOKEN": "sk-your-api-key-here"
        }
      }
    },
    {
      name: "PackyCode",
      websiteUrl: "https://www.packycode.com",
      settingsConfig: {
        "env": {
          "ANTHROPIC_BASE_URL": "https://api.packycode.com",
          "ANTHROPIC_AUTH_TOKEN": "sk-your-api-key-here"
        }
      }
    },
    {
      name: "YesCode",
      websiteUrl: "https://yes.vg",
      settingsConfig: {
        "env": {
          "ANTHROPIC_BASE_URL": "https://co.yes.vg",
          "ANTHROPIC_AUTH_TOKEN": "cr-your-api-key-here"
        }
      }
    }
  ];

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setError("");

    if (!formData.name) {
      setError("请填写供应商名称");
      return;
    }

    if (!formData.settingsConfig.trim()) {
      setError("请填写配置内容");
      return;
    }

    let settingsConfig: object;
    
    try {
      settingsConfig = JSON.parse(formData.settingsConfig);
    } catch (err) {
      setError("配置JSON格式错误，请检查语法");
      return;
    }

    onAdd({
      name: formData.name,
      websiteUrl: formData.websiteUrl,
      settingsConfig
    });
  };

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
  ) => {
    const { name, value } = e.target;
    setFormData({
      ...formData,
      [name]: value,
    });
  };

  const applyPreset = (preset: typeof presets[0]) => {
    setFormData({
      name: preset.name,
      websiteUrl: preset.websiteUrl,
      settingsConfig: JSON.stringify(preset.settingsConfig, null, 2)
    });
  };

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <h2>添加新供应商</h2>

        {error && <div className="error-message">{error}</div>}

        <div className="presets">
          <label>快速选择模板：</label>
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
              placeholder="例如：Anthropic 官方"
              required
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
              添加
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default AddProviderModal;
