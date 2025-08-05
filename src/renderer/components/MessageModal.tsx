import React from 'react'
import './AddProviderModal.css'

interface MessageModalProps {
  title: string
  message: string
  type?: 'success' | 'error' | 'info'
  onClose: () => void
}

const MessageModal: React.FC<MessageModalProps> = ({
  title,
  message,
  type = 'info',
  onClose
}) => {
  const getIcon = () => {
    switch (type) {
      case 'success':
        return '✅'
      case 'error':
        return '❌'
      default:
        return 'ℹ️'
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <h2>{getIcon()} {title}</h2>
        <p>{message}</p>
        
        <div className="form-actions">
          <button type="button" className="submit-btn" onClick={onClose}>
            确定
          </button>
        </div>
      </div>
    </div>
  )
}

export default MessageModal