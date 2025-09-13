import React from "react";

interface CodexConfigEditorProps {
  authValue: string;
  configValue: string;
  onAuthChange: (value: string) => void;
  onConfigChange: (value: string) => void;
  onAuthBlur?: () => void;
}

const CodexConfigEditor: React.FC<CodexConfigEditorProps> = ({
  authValue,
  configValue,
  onAuthChange,
  onConfigChange,
  onAuthBlur,
}) => {
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
        />
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Codex auth.json 配置内容
        </p>
      </div>

      <div className="space-y-2">
        <label
          htmlFor="codexConfig"
          className="block text-sm font-medium text-gray-900 dark:text-gray-100"
        >
          config.toml (TOML)
        </label>
        <textarea
          id="codexConfig"
          value={configValue}
          onChange={(e) => onConfigChange(e.target.value)}
          placeholder=""
          rows={8}
          className="w-full px-3 py-2 border border-gray-200 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20 focus:border-blue-500 dark:focus:border-blue-400 transition-colors resize-y min-h-[10rem]"
        />
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Codex config.toml 配置内容
        </p>
      </div>
    </div>
  );
};

export default CodexConfigEditor;
