import React, { useState, useEffect } from "react";
import { X, Save } from "lucide-react";
import { extractBaseUrlFromToml } from "../../utils/providerConfigUtils";

interface CodexConfigEditorProps {
  authValue: string;
  configValue: string;
  onAuthChange: (value: string) => void;
  onConfigChange: (value: string) => void;
  onAuthBlur?: () => void;
  useCommonConfig: boolean;
  onCommonConfigToggle: (checked: boolean) => void;
  commonConfigSnippet: string;
  onCommonConfigSnippetChange: (value: string) => void;
  commonConfigError: string;
  authError: string;
}

const CodexConfigEditor: React.FC<CodexConfigEditorProps> = ({
  authValue,
  configValue,
  onAuthChange,
  onConfigChange,
  onAuthBlur,
  useCommonConfig,
  onCommonConfigToggle,
  commonConfigSnippet,
  onCommonConfigSnippetChange,
  commonConfigError,
  authError,
}) => {
  const [isCommonConfigModalOpen, setIsCommonConfigModalOpen] = useState(false);
  const [writeVscodeConfig, setWriteVscodeConfig] = useState(false);
  const [vscodeError, setVscodeError] = useState("");
  const [vscodeSuccess, setVscodeSuccess] = useState("");

  useEffect(() => {
    if (commonConfigError && !isCommonConfigModalOpen) {
      setIsCommonConfigModalOpen(true);
    }
  }, [commonConfigError, isCommonConfigModalOpen]);

  useEffect(() => {
    if (!vscodeSuccess) return;
    const timer = window.setTimeout(() => {
      setVscodeSuccess("");
    }, 3000);
    return () => window.clearTimeout(timer);
  }, [vscodeSuccess]);

  // 监听 writeVscodeConfig 变化并执行写入
  useEffect(() => {
    if (!writeVscodeConfig) return;

    const performWrite = async () => {
      setVscodeError("");
      setVscodeSuccess("");

      if (typeof window === "undefined" || !window.api?.writeVscodeSettings) {
        setVscodeError("当前环境暂不支持写入 VS Code 配置");
        setWriteVscodeConfig(false);
        return;
      }

      const trimmed = configValue.trim();
      if (!trimmed) {
        setVscodeError("请先填写 config.toml，再写入 VS Code 配置");
        setWriteVscodeConfig(false);
        return;
      }

      const baseUrl = extractBaseUrlFromToml(trimmed);
      if (!baseUrl) {
        setVscodeError("未在 config.toml 中找到 base_url 字段");
        setWriteVscodeConfig(false);
        return;
      }

      try {
        const success = await window.api.writeVscodeSettings(baseUrl);
        if (success) {
          setVscodeSuccess("已写入 VS Code 配置");
        } else {
          setVscodeError("写入 VS Code 配置失败，请稍后重试");
          setWriteVscodeConfig(false);
        }
      } catch (error) {
        setVscodeError(`写入 VS Code 配置失败: ${String(error)}`);
        setWriteVscodeConfig(false);
      }
    };

    performWrite();
  }, [writeVscodeConfig, configValue]);

  // 支持按下 ESC 关闭弹窗
  useEffect(() => {
    if (!isCommonConfigModalOpen) return;
    
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        closeModal();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [isCommonConfigModalOpen]);

  const closeModal = () => {
    setIsCommonConfigModalOpen(false);
  };

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <label
          htmlFor="codexAuth"
          className="block text-sm font-medium text-gray-900 dark:text-gray-100"
        >
          auth.json (JSON) *
        </label>
        <textarea
          id="codexAuth"
          value={authValue}
          onChange={(e) => onAuthChange(e.target.value)}
          onBlur={onAuthBlur}
          placeholder={`{
  "OPENAI_API_KEY": "sk-your-api-key-here"
}`}
          rows={6}
          required
          className="w-full px-3 py-2 border border-gray-200 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20 focus:border-blue-500 dark:focus:border-blue-400 transition-colors resize-y min-h-[8rem]"
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="none"
          spellCheck={false}
          lang="en"
          inputMode="text"
          data-gramm="false"
          data-gramm_editor="false"
          data-enable-grammarly="false"
        />
        {authError && (
          <p className="text-xs text-red-500 dark:text-red-400">
            {authError}
          </p>
        )}
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Codex auth.json 配置内容
        </p>
      </div>

      <div className="space-y-2">
        <div className="flex items-start justify-between">
          <label
            htmlFor="codexConfig"
            className="block text-sm font-medium text-gray-900 dark:text-gray-100"
          >
            config.toml (TOML)
          </label>
          
          {/* 右侧对齐的双列布局 - 使用flex而非grid以更好控制宽度 */}
          <div className="flex gap-3">
            {/* 左列：VS Code 配置 */}
            <div className="flex flex-col items-end space-y-1">
              <label className="inline-flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400 cursor-pointer">
                <input
                  type="checkbox"
                  checked={writeVscodeConfig}
                  onChange={(e) => setWriteVscodeConfig(e.target.checked)}
                  className="w-4 h-4 text-blue-500 bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 rounded focus:ring-blue-500 dark:focus:ring-blue-400 focus:ring-2"
                />
                写入 VS Code 配置
              </label>
              <div className="h-4 w-48 flex justify-end">
                {vscodeSuccess && !vscodeError && (
                  <p className="text-xs text-emerald-600 dark:text-emerald-400 text-right truncate">
                    {vscodeSuccess}
                  </p>
                )}
                {vscodeError && (
                  <p className="text-xs text-red-500 dark:text-red-400 text-right truncate" title={vscodeError}>
                    {vscodeError}
                  </p>
                )}
              </div>
            </div>
            
            {/* 右列：通用配置 - 不设置宽度，让内容自然收缩 */}
            <div className="flex flex-col items-end space-y-1">
              <label className="inline-flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400 cursor-pointer">
                <input
                  type="checkbox"
                  checked={useCommonConfig}
                  onChange={(e) => onCommonConfigToggle(e.target.checked)}
                  className="w-4 h-4 text-blue-500 bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 rounded focus:ring-blue-500 dark:focus:ring-blue-400 focus:ring-2"
                />
                写入通用配置
              </label>
              <button
                type="button"
                onClick={() => setIsCommonConfigModalOpen(true)}
                className="text-xs text-blue-500 dark:text-blue-400 hover:underline whitespace-nowrap"
              >
                编辑通用配置
              </button>
              {commonConfigError && !isCommonConfigModalOpen && (
                <p className="text-xs text-red-500 dark:text-red-400 mt-1 max-w-[120px] truncate" title={commonConfigError}>
                  {commonConfigError}
                </p>
              )}
            </div>
          </div>
        </div>
        
        <textarea
          id="codexConfig"
          value={configValue}
          onChange={(e) => onConfigChange(e.target.value)}
          placeholder=""
          rows={8}
          className="w-full px-3 py-2 border border-gray-200 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20 focus:border-blue-500 dark:focus:border-blue-400 transition-colors resize-y min-h-[10rem]"
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="none"
          spellCheck={false}
          lang="en"
          inputMode="text"
          data-gramm="false"
          data-gramm_editor="false"
          data-enable-grammarly="false"
        />
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Codex config.toml 配置内容
        </p>
      </div>

      {isCommonConfigModalOpen && (
        <div 
          className="fixed inset-0 z-50 flex items-center justify-center"
          onMouseDown={(e) => {
            if (e.target === e.currentTarget) closeModal();
          }}
        >
          {/* Backdrop - 统一背景样式 */}
          <div className="absolute inset-0 bg-black/50 dark:bg-black/70 backdrop-blur-sm" />
          
          {/* Modal - 统一窗口样式 */}
          <div className="relative bg-white dark:bg-gray-900 rounded-xl shadow-lg max-w-2xl w-full mx-4 max-h-[90vh] overflow-hidden flex flex-col">
            {/* Header - 统一标题栏样式 */}
            <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-800">
              <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">
                编辑 Codex 通用配置片段
              </h2>
              <button
                type="button"
                onClick={closeModal}
                className="p-1 text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-md transition-colors"
                aria-label="关闭"
              >
                <X size={18} />
              </button>
            </div>
            
            {/* Content - 统一内容区域样式 */}
            <div className="flex-1 overflow-auto p-6 space-y-4">
              <p className="text-sm text-gray-500 dark:text-gray-400">
                该片段会在勾选"写入通用配置"时追加到 config.toml 末尾
              </p>
              <textarea
                value={commonConfigSnippet}
                onChange={(e) => onCommonConfigSnippetChange(e.target.value)}
                placeholder={`# Common Codex config
# Add your common TOML configuration here`}
                rows={12}
                className="w-full px-3 py-2 border border-gray-200 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20 focus:border-blue-500 dark:focus:border-blue-400 transition-colors resize-y"
                autoComplete="off"
                autoCorrect="off"
                autoCapitalize="none"
                spellCheck={false}
                lang="en"
                inputMode="text"
                data-gramm="false"
                data-gramm_editor="false"
                data-enable-grammarly="false"
              />
              {commonConfigError && (
                <p className="text-sm text-red-500 dark:text-red-400">
                  {commonConfigError}
                </p>
              )}
            </div>
            
            {/* Footer - 统一底部按钮样式 */}
            <div className="flex items-center justify-end gap-3 p-6 border-t border-gray-200 dark:border-gray-800 bg-gray-100 dark:bg-gray-800">
              <button
                type="button"
                onClick={closeModal}
                className="px-4 py-2 text-sm font-medium text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-white dark:hover:bg-gray-700 rounded-lg transition-colors"
              >
                取消
              </button>
              <button
                type="button"
                onClick={closeModal}
                className="px-4 py-2 bg-blue-500 dark:bg-blue-600 text-white rounded-lg hover:bg-blue-600 dark:hover:bg-blue-700 transition-colors text-sm font-medium flex items-center gap-2"
              >
                <Save className="w-4 h-4" />
                保存
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default CodexConfigEditor;
