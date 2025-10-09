import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { X, Save, AlertCircle } from "lucide-react";
import { McpServer } from "../../types";
import { buttonStyles, inputStyles } from "../../lib/styles";
import McpWizardModal from "./McpWizardModal";

interface McpFormModalProps {
  editingId?: string;
  initialData?: McpServer;
  onSave: (id: string, server: McpServer) => void;
  onClose: () => void;
}

/**
 * 验证 JSON 格式
 */
const validateJson = (text: string): string => {
  if (!text.trim()) return "";
  try {
    const parsed = JSON.parse(text);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return "JSON 必须是对象";
    }
    return "";
  } catch {
    return "JSON 格式错误";
  }
};

/**
 * MCP 表单模态框组件（简化版）
 * 仅包含：标题（必填）、描述（可选）、JSON 配置（可选，带格式校验）
 */
const McpFormModal: React.FC<McpFormModalProps> = ({
  editingId,
  initialData,
  onSave,
  onClose,
}) => {
  const { t } = useTranslation();
  const [formId, setFormId] = useState(editingId || "");
  const [formDescription, setFormDescription] = useState("");
  const [formJson, setFormJson] = useState(
    initialData ? JSON.stringify(initialData, null, 2) : "",
  );
  const [jsonError, setJsonError] = useState("");
  const [saving, setSaving] = useState(false);
  const [isWizardOpen, setIsWizardOpen] = useState(false);

  // 编辑模式下禁止修改 ID
  const isEditing = !!editingId;

  const handleJsonChange = (value: string) => {
    setFormJson(value);
    setJsonError(validateJson(value));
  };

  const handleWizardApply = (json: string) => {
    setFormJson(json);
    setJsonError(validateJson(json));
  };

  const handleSubmit = async () => {
    if (!formId.trim()) {
      alert(t("mcp.error.idRequired"));
      return;
    }

    // 验证 JSON
    const currentJsonError = validateJson(formJson);
    setJsonError(currentJsonError);
    if (currentJsonError) {
      alert(t("mcp.error.jsonInvalid"));
      return;
    }

    setSaving(true);
    try {
      let server: McpServer;
      if (formJson.trim()) {
        // 解析 JSON 配置
        server = JSON.parse(formJson) as McpServer;
      } else {
        // 空 JSON 时提供默认值
        server = {
          type: "stdio",
          command: "",
          args: [],
        };
      }

      // 保留原有的 enabled 状态
      if (initialData?.enabled !== undefined) {
        server.enabled = initialData.enabled;
      }

      onSave(formId.trim(), server);
    } catch (error) {
      alert(t("mcp.error.saveFailed"));
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
          {/* ID (标题) */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              {t("mcp.form.title")} <span className="text-red-500">*</span>
            </label>
            <input
              className={inputStyles.text}
              placeholder={t("mcp.form.titlePlaceholder")}
              value={formId}
              onChange={(e) => setFormId(e.target.value)}
              disabled={isEditing}
            />
          </div>

          {/* Description (描述) */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              {t("mcp.form.description")}
            </label>
            <input
              className={inputStyles.text}
              placeholder={t("mcp.form.descriptionPlaceholder")}
              value={formDescription}
              onChange={(e) => setFormDescription(e.target.value)}
            />
          </div>

          {/* JSON 配置 */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                {t("mcp.form.jsonConfig")}
              </label>
              <button
                type="button"
                onClick={() => setIsWizardOpen(true)}
                className="text-xs text-blue-500 dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300 transition-colors"
              >
                {t("mcp.form.useWizard")}
              </button>
            </div>
            <textarea
              className={`${inputStyles.text} h-64 resize-none font-mono text-xs`}
              placeholder={t("mcp.form.jsonPlaceholder")}
              value={formJson}
              onChange={(e) => handleJsonChange(e.target.value)}
            />
            {jsonError && (
              <div className="flex items-center gap-2 mt-2 text-red-500 dark:text-red-400 text-sm">
                <AlertCircle size={16} />
                <span>{jsonError}</span>
              </div>
            )}
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

      {/* Wizard Modal */}
      <McpWizardModal
        isOpen={isWizardOpen}
        onClose={() => setIsWizardOpen(false)}
        onApply={handleWizardApply}
      />
    </div>
  );
};

export default McpFormModal;
