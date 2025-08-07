import { useState, useEffect, useRef } from "react";
import { Provider } from "../shared/types";
import ProviderList from "./components/ProviderList";
import AddProviderModal from "./components/AddProviderModal";
import EditProviderModal from "./components/EditProviderModal";
import ImportConfigModal from "./components/ImportConfigModal";
import { ConfirmDialog } from "./components/ConfirmDialog";
import "./App.css";

function App() {
  const [providers, setProviders] = useState<Record<string, Provider>>({});
  const [currentProviderId, setCurrentProviderId] = useState<string>("");
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [isImportModalOpen, setIsImportModalOpen] = useState(false);
  const [configPath, setConfigPath] = useState<string>("");
  const [editingProviderId, setEditingProviderId] = useState<string | null>(
    null
  );
  const [notification, setNotification] = useState<{
    message: string;
    type: "success" | "error";
  } | null>(null);
  const [isNotificationVisible, setIsNotificationVisible] = useState(false);
  const [confirmDialog, setConfirmDialog] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    onConfirm: () => void;
  } | null>(null);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);

  // 设置通知的辅助函数
  const showNotification = (
    message: string,
    type: "success" | "error",
    duration = 3000
  ) => {
    // 清除之前的定时器
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }

    // 立即显示通知
    setNotification({ message, type });
    setIsNotificationVisible(true);

    // 设置淡出定时器
    timeoutRef.current = setTimeout(() => {
      setIsNotificationVisible(false);
      // 等待淡出动画完成后清除通知
      setTimeout(() => {
        setNotification(null);
        timeoutRef.current = null;
      }, 300); // 与CSS动画时间匹配
    }, duration);
  };

  // 加载供应商列表
  useEffect(() => {
    loadProviders();
    loadConfigPath();
  }, []);

  // 清理定时器
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  const loadProviders = async () => {
    const loadedProviders = await window.electronAPI.getProviders();
    const currentId = await window.electronAPI.getCurrentProvider();
    setProviders(loadedProviders);
    setCurrentProviderId(currentId);
    
    // 如果供应商列表为空，自动弹出导入配置对话框
    if (Object.keys(loadedProviders).length === 0) {
      setIsImportModalOpen(true);
    }
  };

  const loadConfigPath = async () => {
    const path = await window.electronAPI.getClaudeCodeConfigPath();
    setConfigPath(path);
  };

  // 生成唯一ID
  const generateId = () => {
    return Date.now().toString(36) + Math.random().toString(36).substr(2, 9);
  };

  const handleAddProvider = async (provider: Omit<Provider, "id">) => {
    const newProvider: Provider = {
      ...provider,
      id: generateId(),
    };
    await window.electronAPI.addProvider(newProvider);
    await loadProviders();
    setIsAddModalOpen(false);
  };

  const handleDeleteProvider = async (id: string) => {
    const provider = providers[id];
    setConfirmDialog({
      isOpen: true,
      title: "删除供应商",
      message: `确定要删除供应商 "${provider?.name}" 吗？此操作无法撤销。`,
      onConfirm: async () => {
        await window.electronAPI.deleteProvider(id);
        await loadProviders();
        setConfirmDialog(null);
        showNotification("供应商删除成功", "success");
      },
    });
  };

  const handleSwitchProvider = async (id: string) => {
    const success = await window.electronAPI.switchProvider(id);
    if (success) {
      setCurrentProviderId(id);
      // 显示重启提示
      showNotification(
        "切换成功！请重启 Claude Code 终端以生效",
        "success",
        2000
      );
    } else {
      showNotification("切换失败，请检查配置", "error");
    }
  };

  const handleEditProvider = async (provider: Provider) => {
    try {
      await window.electronAPI.updateProvider(provider);
      await loadProviders();
      setEditingProviderId(null);
      // 显示编辑成功提示
      showNotification("供应商配置已保存", "success", 2000);
    } catch (error) {
      console.error("更新供应商失败:", error);
      setEditingProviderId(null);
      showNotification("保存失败，请重试", "error");
    }
  };

  const handleImportCurrentConfig = async (name: string) => {
    try {
      const result = await window.electronAPI.importCurrentConfig(name)
      
      if (result.success) {
        await loadProviders()
        setIsImportModalOpen(false)
        showNotification(`成功导入当前配置为供应商: ${name}`, "success", 3000)
      } else {
        showNotification("导入失败，请检查当前是否有有效的配置文件", "error")
      }
    } catch (error) {
      console.error('导入配置失败:', error)
      showNotification("导入配置时发生错误", "error")
    }
  }

  const handleSelectConfigFile = async () => {
    const selectedPath = await window.electronAPI.selectConfigFile();
    if (selectedPath) {
      setConfigPath(selectedPath);
    }
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>Claude Code 供应商切换器</h1>
        <div className="header-actions">
          <button className="import-btn" onClick={() => setIsImportModalOpen(true)}>
            导入当前配置
          </button>
          <button className="add-btn" onClick={() => setIsAddModalOpen(true)}>
            添加供应商
          </button>
        </div>
      </header>

      <main className="app-main">
        <div className="provider-section">
          {/* 浮动通知组件 */}
          {notification && (
            <div
              className={`notification-floating ${
                notification.type === "error"
                  ? "notification-error"
                  : "notification-success"
              } ${isNotificationVisible ? "fade-in" : "fade-out"}`}
            >
              {notification.message}
            </div>
          )}

          <ProviderList
            providers={providers}
            currentProviderId={currentProviderId}
            onSwitch={handleSwitchProvider}
            onDelete={handleDeleteProvider}
            onEdit={setEditingProviderId}
          />
        </div>

        {configPath && (
          <div className="config-path">
            <span>配置文件位置: {configPath}</span>
            <button
              className="browse-btn"
              onClick={handleSelectConfigFile}
              title="浏览选择配置文件"
            >
              浏览
            </button>
          </div>
        )}
      </main>

      {isAddModalOpen && (
        <AddProviderModal
          onAdd={handleAddProvider}
          onClose={() => setIsAddModalOpen(false)}
        />
      )}

      {isImportModalOpen && (
        <ImportConfigModal
          onImport={handleImportCurrentConfig}
          onClose={() => setIsImportModalOpen(false)}
          isEmpty={Object.keys(providers).length === 0}
        />
      )}

      {editingProviderId && providers[editingProviderId] && (
        <EditProviderModal
          provider={providers[editingProviderId]}
          onSave={handleEditProvider}
          onClose={() => setEditingProviderId(null)}
        />
      )}

      {confirmDialog && (
        <ConfirmDialog
          isOpen={confirmDialog.isOpen}
          title={confirmDialog.title}
          message={confirmDialog.message}
          onConfirm={confirmDialog.onConfirm}
          onCancel={() => setConfirmDialog(null)}
        />
      )}
    </div>
  );
}

export default App;
