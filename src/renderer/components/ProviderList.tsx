import React from 'react'
import { Provider } from '../../shared/types'
import './ProviderList.css'

interface ProviderListProps {
  providers: Record<string, Provider>
  currentProviderId: string
  onSwitch: (id: string) => void
  onDelete: (id: string) => void
  onEdit: (id: string) => void
  isLoading?: boolean
}

const ProviderList: React.FC<ProviderListProps> = ({
  providers,
  currentProviderId,
  onSwitch,
  onDelete,
  onEdit,
  isLoading = false
}) => {
  return (
    <div className="provider-list">
      {Object.values(providers).length === 0 ? (
        <div className="empty-state">
          <p>还没有添加任何供应商</p>
          <p>点击右上角的"添加供应商"按钮开始</p>
        </div>
      ) : (
        <div className="provider-items">
          {Object.values(providers).map((provider) => {
            const isCurrent = provider.id === currentProviderId
            
            return (
              <div 
                key={provider.id} 
                className={`provider-item ${isCurrent ? 'current' : ''}`}
              >
                <div className="provider-info">
                  <div className="provider-name">
                    <input
                      type="radio"
                      name="provider"
                      checked={isCurrent}
                      onChange={() => onSwitch(provider.id)}
                    />
                    <span>{provider.name}</span>
                    {isCurrent && <span className="current-badge">当前使用</span>}
                  </div>
                  <div className="provider-url">{provider.apiUrl}</div>
                </div>
                
                <div className="provider-actions">
                  <button 
                    className="enable-btn"
                    onClick={() => onSwitch(provider.id)}
                    disabled={isCurrent || isLoading}
                  >
                    {isLoading ? '处理中...' : '启用'}
                  </button>
                  <button 
                    className="edit-btn"
                    onClick={() => onEdit(provider.id)}
                    disabled={isLoading}
                  >
                    编辑
                  </button>
                  <button 
                    className="delete-btn"
                    onClick={() => onDelete(provider.id)}
                    disabled={isCurrent || isLoading}
                  >
                    删除
                  </button>
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}

export default ProviderList