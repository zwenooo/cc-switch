import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { X, Save, Wrench } from "lucide-react";
import { McpServer } from "../../types";
import { buttonStyles, inputStyles } from "../../lib/styles";

interface McpFormModalProps {
  editingId?: string;
  initialData?: McpServer;
  onSave: (id: string, server: McpServer) => void;
  onClose: () => void;
}

const parseEnvText = (text: string): Record<string, string> => {
  const lines = text
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
  const env: Record<string, string> = {};
  for (const l of lines) {
    const idx = l.indexOf("=");
    if (idx > 0) {
      const k = l.slice(0, idx).trim();
      const v = l.slice(idx + 1).trim();
      if (k) env[k] = v;
    }
  }
  return env;
};

const formatEnvText = (env?: Record<string, string>): string => {
  if (!env) return "";
  return Object.entries(env)
    .map(([k, v]) => `${k}=${v}`)
    .join("\n");
};

/**
 * MCP 表单模态框组件
 * 用于添加或编辑 MCP 服务器
 */
const McpFormModal: React.FC<McpFormModalProps> = ({
  editingId,
  initialData,
  onSave,
  onClose,
}) => {
  const { t } = useTranslation();
  const [formId, setFormId] = useState(editingId || "");
  const [formType, setFormType] = useState<"stdio" | "sse">(
    initialData?.type || "stdio",
  );
  const [formCommand, setFormCommand] = useState(initialData?.command || "");
  const [formArgsText, setFormArgsText] = useState(
    (initialData?.args || []).join(" "),
  );
  const [formEnvText, setFormEnvText] = useState(
    formatEnvText(initialData?.env),
  );
  const [formCwd, setFormCwd] = useState(initialData?.cwd || "");
  const [saving, setSaving] = useState(false);

  // 编辑模式下禁止修改 ID
  const isEditing = !!editingId;

  const handleValidateCommand = async () => {
    if (!formCommand) return;
    try {
      const ok = await window.api.validateMcpCommand(formCommand.trim());
      const message = ok ? t("mcp.validation.ok") : t("mcp.validation.fail");
      // 这里简单使用 alert，实际项目中应该使用 notification 系统
      alert(message);
    } catch (_error) {
      alert(t("mcp.validation.fail"));
    }
  };

  const handleSubmit = async () => {
    if (!formId.trim()) {
      alert(t("mcp.error.idRequired"));
      return;
    }
    if (!formCommand.trim()) {
      alert(t("mcp.error.commandRequired"));
      return;
    }

    setSaving(true);
    try {
      const server: McpServer = {
        type: formType,
        command: formCommand.trim(),
        args: formArgsText
          .split(/\s+/)
          .map((s) => s.trim())
          .filter((s) => s.length > 0),
        env: parseEnvText(formEnvText),
        ...(formCwd ? { cwd: formCwd } : {}),
        // 保留原有的 enabled 状态
        ...(initialData?.enabled !== undefined
          ? { enabled: initialData.enabled }
          : {}),
      };

      onSave(formId.trim(), server);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative bg-white dark:bg-gray-900 rounded-xl shadow-lg max-w-2xl w-full mx-4 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-800">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            {isEditing ? t("mcp.editServer") : t("mcp.addServer")}
          </h3>
          <button
            onClick={onClose}
            className="p-1 text-gray-500 hover:text-gray-900 hover:bg-gray-100 dark:text-gray-400 dark:hover:text-gray-100 dark:hover:bg-gray-800 rounded-md transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-4">
          {/* ID */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              {t("mcp.id")}
            </label>
            <input
              className={inputStyles.text}
              placeholder="my-mcp"
              value={formId}
              onChange={(e) => setFormId(e.target.value)}
              disabled={isEditing}
            />
          </div>

          {/* Type & CWD */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                {t("mcp.type")}
              </label>
              <select
                className={inputStyles.select}
                value={formType}
                onChange={(e) => setFormType(e.target.value as "stdio" | "sse")}
              >
                <option value="stdio">stdio</option>
                <option value="sse">sse</option>
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                {t("mcp.cwd")}
              </label>
              <input
                className={inputStyles.text}
                placeholder="/path/to/project"
                value={formCwd}
                onChange={(e) => setFormCwd(e.target.value)}
              />
            </div>
          </div>

          {/* Command */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              {t("mcp.command")}
            </label>
            <div className="flex gap-2">
              <input
                className={inputStyles.text}
                placeholder="uvx"
                value={formCommand}
                onChange={(e) => setFormCommand(e.target.value)}
              />
              <button
                type="button"
                onClick={handleValidateCommand}
                className="px-3 py-2 rounded-md bg-emerald-500 text-white hover:bg-emerald-600 dark:bg-emerald-600 dark:hover:bg-emerald-700 text-sm inline-flex items-center gap-1 flex-shrink-0 transition-colors"
              >
                <Wrench size={16} /> {t("mcp.validateCommand")}
              </button>
            </div>
          </div>

          {/* Args */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              {t("mcp.args")}
            </label>
            <input
              className={inputStyles.text}
              placeholder={t("mcp.argsPlaceholder")}
              value={formArgsText}
              onChange={(e) => setFormArgsText(e.target.value)}
            />
          </div>

          {/* Env */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              {t("mcp.env")}
            </label>
            <textarea
              className={`${inputStyles.text} h-24 resize-none`}
              placeholder={t("mcp.envPlaceholder")}
              value={formEnvText}
              onChange={(e) => setFormEnvText(e.target.value)}
            />
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 p-6 border-t border-gray-200 dark:border-gray-800">
          <button onClick={onClose} className={buttonStyles.secondary}>
            {t("common.cancel")}
          </button>
          <button
            onClick={handleSubmit}
            disabled={saving}
            className={buttonStyles.primary}
          >
            <Save size={16} />
            {saving
              ? t("common.saving")
              : isEditing
                ? t("common.save")
                : t("common.add")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default McpFormModal;
