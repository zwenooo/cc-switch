import { AppType } from "../lib/tauri-api";
import { Terminal, Code2 } from "lucide-react";

interface AppSwitcherProps {
  activeApp: AppType;
  onSwitch: (app: AppType) => void;
}

export function AppSwitcher({ activeApp, onSwitch }: AppSwitcherProps) {
  const handleSwitch = (app: AppType) => {
    if (app === activeApp) return;
    onSwitch(app);
  };

  return (
    <div className="inline-flex bg-gray-100 rounded-lg p-1 gap-1">
      <button
        type="button"
        onClick={() => handleSwitch("claude")}
        className={`inline-flex items-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-all duration-200 ${
          activeApp === "claude"
            ? "bg-white text-gray-900 shadow-sm"
            : "text-gray-500 hover:text-gray-900 hover:bg-white/50"
        }`}
      >
        <Code2 size={16} />
        <span>Claude Code</span>
      </button>

      <button
        type="button"
        onClick={() => handleSwitch("codex")}
        className={`inline-flex items-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-all duration-200 ${
          activeApp === "codex"
            ? "bg-white text-gray-900 shadow-sm"
            : "text-gray-500 hover:text-gray-900 hover:bg-white/50"
        }`}
      >
        <Terminal size={16} />
        <span>Codex</span>
      </button>
    </div>
  );
}
