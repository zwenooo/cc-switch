import { useState, useEffect, useRef } from "react";
import { Provider } from "./types";
import { AppType } from "./lib/tauri-api";
import ProviderList from "./components/ProviderList";
import AddProviderModal from "./components/AddProviderModal";
import EditProviderModal from "./components/EditProviderModal";
import { ConfirmDialog } from "./components/ConfirmDialog";
import "./App.css";

function App() {
  const [activeApp, setActiveApp] = useState<AppType>("claude");
  const [providers, setProviders] = useState<Record<string, Provider>>({});
  const [currentProviderId, setCurrentProviderId] = useState<string>("");
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [configStatus, setConfigStatus] = useState<{
    exists: boolean;
    path: string;
  } | null>(null);
  const [editingProviderId, setEditingProviderId] = useState<string | null>(
    null,
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
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 设置通知的辅助函数
  const showNotification = (
    message: string,
    type: "success" | "error",
    duration = 3000,
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
    loadConfigStatus();
  }, [activeApp]); // 当切换应用时重新加载

  // 清理定时器
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  const loadProviders = async () => {
    const loadedProviders = await window.api.getProviders(activeApp);
    const currentId = await window.api.getCurrentProvider(activeApp);
    setProviders(loadedProviders);
    setCurrentProviderId(currentId);

    // 如果供应商列表为空，尝试自动导入现有配置为"default"供应商
    if (Object.keys(loadedProviders).length === 0) {
      await handleAutoImportDefault();
    }
  };

  const loadConfigStatus = async () => {
    const status = await window.api.getConfigStatus(activeApp);
    setConfigStatus({
      exists: Boolean(status?.exists),
      path: String(status?.path || ""),
    });
  };

  // 生成唯一ID
  const generateId = () => {
    return crypto.randomUUID();
  };

  const handleAddProvider = async (provider: Omit<Provider, "id">) => {
    const newProvider: Provider = {
      ...provider,
      id: generateId(),
    };
    await window.api.addProvider(newProvider, activeApp);
    await loadProviders();
    setIsAddModalOpen(false);
  };

  const handleEditProvider = async (provider: Provider) => {
    try {
      await window.api.updateProvider(provider, activeApp);
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

  const handleDeleteProvider = async (id: string) => {
    const provider = providers[id];
    setConfirmDialog({
      isOpen: true,
      title: "删除供应商",
      message: `确定要删除供应商 "${provider?.name}" 吗？此操作无法撤销。`,
      onConfirm: async () => {
        await window.api.deleteProvider(id, activeApp);
        await loadProviders();
        setConfirmDialog(null);
        showNotification("供应商删除成功", "success");
      },
    });
  };

  const handleSwitchProvider = async (id: string) => {
    const success = await window.api.switchProvider(id, activeApp);
    if (success) {
      setCurrentProviderId(id);
      // 显示重启提示
      const appName = activeApp === "claude" ? "Claude Code" : "Codex";
      showNotification(
        `切换成功！请重启 ${appName} 终端以生效`,
        "success",
        2000,
      );
    } else {
      showNotification("切换失败，请检查配置", "error");
    }
  };

  // 自动导入现有配置为"default"供应商
  const handleAutoImportDefault = async () => {
    try {
      const result = await window.api.importCurrentConfigAsDefault(activeApp);

      if (result.success) {
        await loadProviders();
        showNotification(
          "已自动导入现有配置为 default 供应商",
          "success",
          3000,
        );
      }
      // 如果导入失败（比如没有现有配置），静默处理，不显示错误
    } catch (error) {
      console.error("自动导入默认配置失败:", error);
      // 静默处理，不影响用户体验
    }
  };

  const handleOpenConfigFolder = async () => {
    await window.api.openConfigFolder(activeApp);
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>{activeApp === "claude" ? "Claude Code" : "Codex"} 供应商切换器</h1>
        <div className="app-tabs">
          <div
            className="segmented"
            role="tablist"
            aria-label="选择应用"
          >
            <span
              className="segmented-thumb"
              style={{
                transform:
                  activeApp === "claude" ? "translateX(0%)" : "translateX(100%)",
              }}
            />
            <button
              type="button"
              role="tab"
              aria-selected={activeApp === "claude"}
              className={`segmented-item ${
                activeApp === "claude" ? "active" : ""
              }`}
              onClick={() => setActiveApp("claude")}
            >
              Claude Code
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={activeApp === "codex"}
              className={`segmented-item ${
                activeApp === "codex" ? "active" : ""
              }`}
              onClick={() => setActiveApp("codex")}
            >
              Codex
            </button>
          </div>
        </div>
        <div className="header-actions">
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
            appType={activeApp}
          />
        </div>

        {configStatus && (
          <div className="config-path">
            <span>
              配置文件位置: {configStatus.path}
              {!configStatus.exists ? "（未创建，切换或保存时会自动创建）" : ""}
            </span>
            <button
              className="browse-btn"
              onClick={handleOpenConfigFolder}
              title="打开配置文件夹"
            >
              打开
            </button>
          </div>
        )}
      </main>

      {isAddModalOpen && (
        <AddProviderModal
          appType={activeApp}
          onAdd={handleAddProvider}
          onClose={() => setIsAddModalOpen(false)}
        />
      )}

      {editingProviderId && providers[editingProviderId] && (
        <EditProviderModal
          appType={activeApp}
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
