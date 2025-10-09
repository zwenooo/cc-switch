import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { X, Plus, Server } from "lucide-react";
import { McpServer, McpStatus } from "../../types";
import McpListItem from "./McpListItem";
import McpFormModal from "./McpFormModal";
import { ConfirmDialog } from "../ConfirmDialog";

interface McpPanelProps {
  onClose: () => void;
  onNotify?: (
    message: string,
    type: "success" | "error",
    duration?: number,
  ) => void;
}

/**
 * MCP 管理面板
 * 采用与主界面一致的设计风格，右上角添加按钮，每个 MCP 占一行
 */
const McpPanel: React.FC<McpPanelProps> = ({ onClose, onNotify }) => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<McpStatus | null>(null);
  const [servers, setServers] = useState<Record<string, McpServer>>({});
  const [loading, setLoading] = useState(true);
  const [isFormOpen, setIsFormOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [confirmDialog, setConfirmDialog] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    onConfirm: () => void;
  } | null>(null);

  const reload = async () => {
    setLoading(true);
    try {
      const s = await window.api.getClaudeMcpStatus();
      setStatus(s);
      const text = await window.api.readClaudeMcpConfig();
      if (text) {
        try {
          const obj = JSON.parse(text);
          const list = (obj?.mcpServers || {}) as Record<string, McpServer>;
          setServers(list);
        } catch (e) {
          console.error("Failed to parse mcp.json", e);
          setServers({});
        }
      } else {
        setServers({});
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    reload();
  }, []);

  const handleToggle = async (id: string, enabled: boolean) => {
    try {
      const server = servers[id];
      if (!server) return;

      const updatedServer = { ...server, enabled };
      await window.api.upsertClaudeMcpServer(id, updatedServer);
      await reload();
      onNotify?.(
        enabled ? t("mcp.msg.enabled") : t("mcp.msg.disabled"),
        "success",
        1500,
      );
    } catch (e: any) {
      onNotify?.(e?.message || t("mcp.error.saveFailed"), "error", 5000);
    }
  };

  const handleEdit = (id: string) => {
    setEditingId(id);
    setIsFormOpen(true);
  };

  const handleAdd = () => {
    setEditingId(null);
    setIsFormOpen(true);
  };

  const handleDelete = (id: string) => {
    setConfirmDialog({
      isOpen: true,
      title: t("mcp.confirm.deleteTitle"),
      message: t("mcp.confirm.deleteMessage", { id }),
      onConfirm: async () => {
        try {
          await window.api.deleteClaudeMcpServer(id);
          await reload();
          setConfirmDialog(null);
          onNotify?.(t("mcp.msg.deleted"), "success", 1500);
        } catch (e: any) {
          onNotify?.(e?.message || t("mcp.error.deleteFailed"), "error", 5000);
        }
      },
    });
  };

  const handleSave = async (id: string, server: McpServer) => {
    try {
      await window.api.upsertClaudeMcpServer(id, server);
      await reload();
      setIsFormOpen(false);
      setEditingId(null);
      onNotify?.(t("mcp.msg.saved"), "success", 1500);
    } catch (e: any) {
      onNotify?.(e?.message || t("mcp.error.saveFailed"), "error", 6000);
      // 继续抛出错误，让表单层可以给到直观反馈（避免被更高层遮挡）
      throw e;
    }
  };

  const handleCloseForm = () => {
    setIsFormOpen(false);
    setEditingId(null);
  };

  const serverEntries = useMemo(() => Object.entries(servers), [servers]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Panel */}
      <div className="relative bg-white dark:bg-gray-900 rounded-xl shadow-lg max-w-4xl w-full mx-4 overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="flex-shrink-0 flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-800">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            {t("mcp.title")}
          </h3>

          <div className="flex items-center gap-3">
            <button
              onClick={handleAdd}
              className="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-md bg-emerald-500 text-white hover:bg-emerald-600 dark:bg-emerald-600 dark:hover:bg-emerald-700 transition-colors"
            >
              <Plus size={16} />
              {t("mcp.add")}
            </button>
            <button
              onClick={onClose}
              className="p-1 text-gray-500 hover:text-gray-900 hover:bg-gray-100 dark:text-gray-400 dark:hover:text-gray-100 dark:hover:bg-gray-800 rounded-md transition-colors"
            >
              <X size={18} />
            </button>
          </div>
        </div>

        {/* Info Section */}
        <div className="flex-shrink-0 px-6 pt-4 pb-2">
          <div className="text-sm text-gray-500 dark:text-gray-400">
            {t("mcp.configPath")}:{" "}
            <span className="text-xs break-all">{status?.userConfigPath}</span>
          </div>
          <div className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            {t("mcp.serverCount", { count: status?.serverCount || 0 })}
          </div>
        </div>

        {/* Content - Scrollable */}
        <div className="flex-1 overflow-y-auto px-6 py-4">
          {loading ? (
            <div className="text-center py-12 text-gray-500 dark:text-gray-400">
              {t("mcp.loading")}
            </div>
          ) : serverEntries.length === 0 ? (
            <div className="text-center py-12">
              <div className="w-16 h-16 mx-auto mb-4 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center">
                <Server
                  size={24}
                  className="text-gray-400 dark:text-gray-500"
                />
              </div>
              <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                {t("mcp.empty")}
              </h3>
              <p className="text-gray-500 dark:text-gray-400 text-sm">
                {t("mcp.emptyDescription")}
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {serverEntries.map(([id, server]) => (
                <McpListItem
                  key={id}
                  id={id}
                  server={server}
                  onToggle={handleToggle}
                  onEdit={handleEdit}
                  onDelete={handleDelete}
                />
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Form Modal */}
      {isFormOpen && (
        <McpFormModal
          editingId={editingId || undefined}
          initialData={editingId ? servers[editingId] : undefined}
          onSave={handleSave}
          onClose={handleCloseForm}
        />
      )}

      {/* Confirm Dialog */}
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
};

export default McpPanel;
