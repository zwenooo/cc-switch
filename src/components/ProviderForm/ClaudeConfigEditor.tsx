import React, { useEffect, useState } from "react";
import JsonEditor from "../JsonEditor";

interface ClaudeConfigEditorProps {
  value: string;
  onChange: (value: string) => void;
  disableCoAuthored: boolean;
  onCoAuthoredToggle: (checked: boolean) => void;
}

const ClaudeConfigEditor: React.FC<ClaudeConfigEditorProps> = ({
  value,
  onChange,
  disableCoAuthored,
  onCoAuthoredToggle,
}) => {
  const [isDarkMode, setIsDarkMode] = useState(false);

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
            checked={disableCoAuthored}
            onChange={(e) => onCoAuthoredToggle(e.target.checked)}
            className="w-4 h-4 text-blue-500 bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 rounded focus:ring-blue-500 dark:focus:ring-blue-400 focus:ring-2"
          />
          禁止 Claude Code 签名
        </label>
      </div>
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
    </div>
  );
};

export default ClaudeConfigEditor;
