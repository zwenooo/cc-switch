import React, { useEffect, useState } from "react";
import JsonEditor from "../JsonEditor";

interface ClaudeConfigEditorProps {
  value: string;
  onChange: (value: string) => void;
  useCommonConfig: boolean;
  onCommonConfigToggle: (checked: boolean) => void;
  commonConfigSnippet: string;
  onCommonConfigSnippetChange: (value: string) => void;
  commonConfigError: string;
}

const ClaudeConfigEditor: React.FC<ClaudeConfigEditorProps> = ({
  value,
  onChange,
  useCommonConfig,
  onCommonConfigToggle,
  commonConfigSnippet,
  onCommonConfigSnippetChange,
  commonConfigError,
}) => {
  const [isDarkMode, setIsDarkMode] = useState(false);
  const [isCommonConfigModalOpen, setIsCommonConfigModalOpen] = useState(false);

  useEffect(() => {
    // 检测暗色模式
    const checkDarkMode = () => {
      setIsDarkMode(document.documentElement.classList.contains("dark"));
    };

    checkDarkMode();

    // 监听暗色模式变化
    const observer = new MutationObserver((mutations) => {
      mutations.forEach((mutation) => {
        if (mutation.attributeName === "class") {
          checkDarkMode();
        }
      });
    });

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });

    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (commonConfigError && !isCommonConfigModalOpen) {
      setIsCommonConfigModalOpen(true);
    }
  }, [commonConfigError, isCommonConfigModalOpen]);

  const closeModal = () => {
    setIsCommonConfigModalOpen(false);
  };
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <label
          htmlFor="settingsConfig"
          className="block text-sm font-medium text-gray-900 dark:text-gray-100"
        >
          Claude Code 配置 (JSON) *
        </label>
        <label className="inline-flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400 cursor-pointer">
          <input
            type="checkbox"
            checked={useCommonConfig}
            onChange={(e) => onCommonConfigToggle(e.target.checked)}
            className="w-4 h-4 text-blue-500 bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 rounded focus:ring-blue-500 dark:focus:ring-blue-400 focus:ring-2"
          />
          写入通用配置
        </label>
      </div>
      <div className="flex items-center justify-end">
        <button
          type="button"
          onClick={() => setIsCommonConfigModalOpen(true)}
          className="text-xs text-blue-500 dark:text-blue-400 hover:underline"
        >
          编辑通用配置
        </button>
      </div>
      {commonConfigError && !isCommonConfigModalOpen && (
        <p className="text-xs text-red-500 dark:text-red-400 text-right">
          {commonConfigError}
        </p>
      )}
      <JsonEditor
        value={value}
        onChange={onChange}
        darkMode={isDarkMode}
        placeholder={`{
  "env": {
    "ANTHROPIC_BASE_URL": "https://your-api-endpoint.com",
    "ANTHROPIC_AUTH_TOKEN": "your-api-key-here"
  }
}`}
        rows={12}
      />
      <p className="text-xs text-gray-500 dark:text-gray-400">
        完整的 Claude Code settings.json 配置内容
      </p>
      {isCommonConfigModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div
            className="absolute inset-0 bg-black/40"
            onClick={closeModal}
          />
          <div className="relative z-10 w-full max-w-2xl mx-4 bg-white dark:bg-gray-900 rounded-xl shadow-lg border border-gray-200 dark:border-gray-800">
            <div className="flex items-center justify-between px-5 py-4 border-b border-gray-200 dark:border-gray-800">
              <div>
                <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
                  编辑通用配置片段
                </h2>
                <p className="text-xs text-gray-500 dark:text-gray-400">
                  该片段会在勾选“写入通用配置”时合并到 settings.json 中
                </p>
              </div>
              <button
                type="button"
                onClick={closeModal}
                className="text-xs text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
              >
                关闭
              </button>
            </div>
            <div className="px-5 py-4 space-y-2">
              <JsonEditor
                value={commonConfigSnippet}
                onChange={onCommonConfigSnippetChange}
                darkMode={isDarkMode}
                rows={12}
              />
              {commonConfigError && (
                <p className="text-xs text-red-500 dark:text-red-400">
                  {commonConfigError}
                </p>
              )}
            </div>
            <div className="flex justify-end gap-2 px-5 py-4 border-t border-gray-200 dark:border-gray-800 bg-gray-50 dark:bg-gray-950">
              <button
                type="button"
                onClick={closeModal}
                className="px-3 py-1.5 text-xs font-medium text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-gray-100"
              >
                完成
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default ClaudeConfigEditor;
