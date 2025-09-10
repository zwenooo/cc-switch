import { X, Sparkles } from "lucide-react";
import { useUpdate } from "../contexts/UpdateContext";

interface UpdateBadgeProps {
  className?: string;
  onClick?: () => void; // 点击徽标的回调（例如打开设置）
}

export function UpdateBadge({ className = "", onClick }: UpdateBadgeProps) {
  const { hasUpdate, updateInfo, isDismissed, dismissUpdate } = useUpdate();
  
  // 如果没有更新或已关闭，不显示
  if (!hasUpdate || isDismissed || !updateInfo) {
    return null;
  }
  
  return (
    <div
      className={`
        flex items-center gap-2 px-3 py-1.5 
        bg-gradient-to-r from-blue-500/20 to-purple-500/20
        border border-blue-500/30
        rounded-full text-xs
        transition-all duration-200
        ${onClick ? "cursor-pointer hover:border-blue-400/50" : ""}
        ${className}
      `}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : -1}
      onClick={onClick}
      onKeyDown={(e) => {
        if (!onClick) return;
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick();
        }
      }}
      >
      <Sparkles className="w-3 h-3 text-blue-400 animate-pulse" />
      <span className="text-gray-200 font-medium">
        新版本 {updateInfo.availableVersion}
      </span>
      <button
        onClick={(e) => {
          e.stopPropagation();
          dismissUpdate();
        }}
        className="
          -mr-1 p-0.5 rounded-full
          hover:bg-white/10 transition-colors
          focus:outline-none focus:ring-2 focus:ring-blue-500/50
        "
        aria-label="关闭更新提醒"
      >
        <X className="w-3 h-3 text-gray-400 hover:text-gray-200" />
      </button>
    </div>
  );
}
