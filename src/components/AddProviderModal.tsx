import React from "react";
import { Provider } from "../types";
import ProviderForm from "./ProviderForm";

interface AddProviderModalProps {
  onAdd: (provider: Omit<Provider, "id">) => void;
  onClose: () => void;
}

const AddProviderModal: React.FC<AddProviderModalProps> = ({
  onAdd,
  onClose,
}) => {
  return (
    <ProviderForm
      title="添加新供应商"
      submitText="添加"
      showPresets={true}
      onSubmit={onAdd}
      onClose={onClose}
    />
  );
};

export default AddProviderModal;
