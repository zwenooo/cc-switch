import React from "react";
import { useTranslation } from "react-i18next";
import { Zap } from "lucide-react";
import { ProviderCategory } from "../../types";
import { ClaudeIcon, CodexIcon } from "../BrandIcons";

interface Preset {
  name: string;
  isOfficial?: boolean;
  category?: ProviderCategory;
}

interface PresetSelectorProps {
  title?: string;
  presets: Preset[];
  selectedIndex: number | null;
  onSelectPreset: (index: number) => void;
  onCustomClick: () => void;
  customLabel?: string;
  renderCustomDescription?: () => React.ReactNode; // 新增：自定义描述渲染
}

const PresetSelector: React.FC<PresetSelectorProps> = ({
  title,
  presets,
  selectedIndex,
  onSelectPreset,
  onCustomClick,
  customLabel,
  renderCustomDescription,
}) => {
  const { t } = useTranslation();

  const getButtonClass = (index: number, preset?: Preset) => {
    const isSelected = selectedIndex === index;
    const baseClass =
      "inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors";

    if (isSelected) {
      if (preset?.isOfficial || preset?.category === "official") {
        // Codex 官方使用黑色背景
        if (preset?.name.includes("Codex")) {
          return `${baseClass} bg-gray-900 text-white`;
        }
        // Claude 官方使用品牌色背景
        return `${baseClass} bg-[#D97757] text-white`;
      }
      return `${baseClass} bg-blue-500 text-white`;
    }

    return `${baseClass} bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700`;
  };

  const getDescription = () => {
    if (selectedIndex === -1) {
      // 如果提供了自定义描述渲染函数，使用它
      if (renderCustomDescription) {
        return renderCustomDescription();
      }
      return t("presetSelector.customDescription");
    }

    if (selectedIndex !== null && selectedIndex >= 0) {
      const preset = presets[selectedIndex];
      return preset?.isOfficial || preset?.category === "official"
        ? t("presetSelector.officialDescription")
        : t("presetSelector.presetDescription");
    }

    return null;
  };

  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">
          {title || t("presetSelector.title")}
        </label>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className={`${getButtonClass(-1)} ${selectedIndex === -1 ? "" : ""}`}
            onClick={onCustomClick}
          >
            {customLabel || t("presetSelector.custom")}
          </button>
          {presets.map((preset, index) => (
            <button
              key={index}
              type="button"
              className={getButtonClass(index, preset)}
              onClick={() => onSelectPreset(index)}
            >
              {(preset.isOfficial || preset.category === "official") && (
                <>
                  {preset.name.includes("Claude") ? (
                    <ClaudeIcon size={14} />
                  ) : preset.name.includes("Codex") ? (
                    <CodexIcon size={14} />
                  ) : (
                    <Zap size={14} />
                  )}
                </>
              )}
              {preset.name}
            </button>
          ))}
        </div>
      </div>
      {getDescription() && (
        <div className="text-sm text-gray-500 dark:text-gray-400">
          {getDescription()}
        </div>
      )}
    </div>
  );
};

export default PresetSelector;
