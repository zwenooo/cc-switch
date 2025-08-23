import React from 'react'
import { Provider } from '../../shared/types'
import ProviderForm from './ProviderForm'

interface EditProviderModalProps {
  provider: Provider
  onSave: (provider: Provider) => void
  onClose: () => void
}

const EditProviderModal: React.FC<EditProviderModalProps> = ({ provider, onSave, onClose }) => {
  const handleSubmit = (data: Omit<Provider, 'id'>) => {
    onSave({
      ...provider,
      ...data
    })
  }

  return (
    <ProviderForm
      title="编辑供应商"
      submitText="保存"
      initialData={provider}
      showPresets={false}
      onSubmit={handleSubmit}
      onClose={onClose}
    />
  )
}

export default EditProviderModal