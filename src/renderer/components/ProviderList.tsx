import React from 'react'
import { Provider, ProviderStatus } from '../../shared/types'
import './ProviderList.css'

interface ProviderListProps {
  providers: Record<string, Provider>
  currentProviderId: string
  statuses: Record<string, ProviderStatus>
  checkingStatus: Record<string, boolean>
  onSwitch: (id: string) => void
  onDelete: (id: string) => void
  onEdit: (id: string) => void
  onCheckStatus: (id: string) => void
}

const ProviderList: React.FC<ProviderListProps> = ({
  providers,
  currentProviderId,
  statuses,
  checkingStatus,
  onSwitch,
  onDelete,
  onEdit,
  onCheckStatus
}) => {
  const formatResponseTime = (time: number) => {
    if (time < 0) return '-'
    return `${time}ms`
  }

  const getStatusIcon = (status?: ProviderStatus) => {
    if (!status) return '⏳'
    return status.isOnline ? '✅' : '❌'
  }

  const getStatusText = (status?: ProviderStatus, isChecking?: boolean) => {
    if (isChecking) return '检查中...'
    if (!status) return '未检查'
    if (status.isOnline) return '正常'
    return status.error || '连接失败'
  }

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
            const status = statuses[provider.id]
            const isChecking = checkingStatus[provider.id]
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
                      disabled={!status?.isOnline}
                    />
                    <span>{provider.name}</span>
                    {isCurrent && <span className="current-badge">当前使用</span>}
                  </div>
                  <div className="provider-url">{provider.apiUrl}</div>
                </div>
                
                <div className="provider-status">
                  <span className="status-icon">{isChecking ? '🔄' : getStatusIcon(status)}</span>
                  <span className="status-text">{getStatusText(status, isChecking)}</span>
                  {status?.isOnline && !isChecking && (
                    <span className="response-time">
                      {formatResponseTime(status.responseTime)}
                    </span>
                  )}
                </div>
                
                <div className="provider-actions">
                  <button 
                    className="check-btn"
                    onClick={() => onCheckStatus(provider.id)}
                    disabled={isChecking}
                  >
                    {isChecking ? '检查中' : '检查状态'}
                  </button>
                  <button 
                    className="enable-btn"
                    onClick={() => onSwitch(provider.id)}
                    disabled={!status?.isOnline || isCurrent}
                  >
                    启用
                  </button>
                  <button 
                    className="edit-btn"
                    onClick={() => onEdit(provider.id)}
                  >
                    编辑
                  </button>
                  <button 
                    className="delete-btn"
                    onClick={() => onDelete(provider.id)}
                    disabled={isCurrent}
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