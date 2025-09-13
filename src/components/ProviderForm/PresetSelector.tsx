import React from "react";
import { Zap } from "lucide-react";
import { ProviderCategory } from "../../types";

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
}

const PresetSelector: React.FC<PresetSelectorProps> = ({
  title = "选择配置类型",
  presets,
  selectedIndex,
  onSelectPreset,
  onCustomClick,
  customLabel = "自定义",
}) => {
  const getButtonClass = (
    index: number,
    isOfficial?: boolean,
    category?: ProviderCategory,
  ) => {
    const isSelected = selectedIndex === index;
    const baseClass =
      "inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors";

    if (isSelected) {
      return isOfficial || category === "official"
        ? `${baseClass} bg-amber-500 text-white`
        : `${baseClass} bg-blue-500 text-white`;
    }

    return `${baseClass} bg-gray-100 text-gray-500 hover:bg-gray-200`;
  };

  const getDescription = () => {
    if (selectedIndex === -1) {
      return "手动配置供应商，需要填写完整的配置信息";
    }

    if (selectedIndex !== null && selectedIndex >= 0) {
      const preset = presets[selectedIndex];
      return preset?.isOfficial || preset?.category === "official"
        ? "官方登录，不需要填写 API Key"
        : "使用预设配置，只需填写 API Key";
    }

    return null;
  };

  return (
    <div className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-900 mb-3">
          {title}
        </label>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className={`${getButtonClass(-1)} ${selectedIndex === -1 ? "" : ""}`}
            onClick={onCustomClick}
          >
            {customLabel}
          </button>
          {presets.map((preset, index) => (
            <button
              key={index}
              type="button"
              className={getButtonClass(
                index,
                preset.isOfficial,
                preset.category,
              )}
              onClick={() => onSelectPreset(index)}
            >
              {(preset.isOfficial || preset.category === "official") && (
                <Zap size={14} />
              )}
              {preset.name}
            </button>
          ))}
        </div>
      </div>
      {getDescription() && (
        <p className="text-sm text-gray-500">{getDescription()}</p>
      )}
    </div>
  );
};

export default PresetSelector;
