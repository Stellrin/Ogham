import React, { useState, useRef, useEffect } from 'react';
import './Toolbar.css';

export type ConversionMode = 's2t' | 't2s';

interface ToolbarProps {
  onConvert: (mode: ConversionMode) => void;
  isConverting: boolean;
  conversionProgress: number;
  onProcessAllImages: () => void;
  isProcessingImages: boolean;
  imageProcessingProgress: number;
  imageProcessingTotal?: number;
  imageProcessingCurrentChapter?: string;
  imageProcessingCurrentImageUrl?: string;
  imageProcessingSuccess?: number;
  imageProcessingFailed?: number;
  imageProcessingSkipped?: number;
  imageProcessingProcessedUnique?: number;
}

export const Toolbar: React.FC<ToolbarProps> = ({
  onConvert,
  isConverting,
  conversionProgress,
  onProcessAllImages,
  isProcessingImages,
  imageProcessingProgress,
  imageProcessingTotal,
  imageProcessingCurrentChapter,
  imageProcessingCurrentImageUrl,
  imageProcessingSuccess = 0,
  imageProcessingFailed = 0,
  imageProcessingSkipped = 0,
  imageProcessingProcessedUnique = 0,
}) => {
  const [isDropdownOpen, setIsDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // 点击外部关闭下拉菜单
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsDropdownOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSelect = (mode: ConversionMode) => {
    setIsDropdownOpen(false);
    onConvert(mode);
  };

  const pendingImages = Math.max((imageProcessingTotal || 0) - imageProcessingProcessedUnique, 0);
  const currentChapterName = imageProcessingCurrentChapter
    ? imageProcessingCurrentChapter.split('/').pop() || imageProcessingCurrentChapter
    : '';

  return (
    <div className="epub-toolbar">
      <div className="toolbar-group" ref={dropdownRef}>
        <div className="dropdown-container">
          <button
            className="toolbar-button"
            onClick={() => setIsDropdownOpen(!isDropdownOpen)}
            disabled={isConverting}
          >
            <span className="toolbar-icon">🔄</span>
            <span className="toolbar-label">简繁转换</span>
            <span className="toolbar-caret">▼</span>
          </button>
          {isDropdownOpen && (
            <div className="dropdown-menu">
              <button
                className="dropdown-item"
                onClick={() => handleSelect('s2t')}
                disabled={isConverting}
              >
                <span className="dropdown-icon">→</span>
                <span>简体 → 繁体</span>
              </button>
              <button
                className="dropdown-item"
                onClick={() => handleSelect('t2s')}
                disabled={isConverting}
              >
                <span className="dropdown-icon">←</span>
                <span>繁体 → 简体</span>
              </button>
            </div>
          )}
        </div>
      </div>

      <div className="toolbar-group">
        <button
          className="toolbar-button"
          onClick={onProcessAllImages}
          disabled={isProcessingImages}
          title="处理整本小说的所有图片链接"
        >
          <span className="toolbar-icon">📚</span>
          <span className="toolbar-label">
            {isProcessingImages && imageProcessingProgress > 0
              ? `处理中 ${imageProcessingProgress}%`
              : '处理图片'}
          </span>
        </button>
      </div>

      {isConverting && (
        <div className="toolbar-progress">
          <div className="progress-bar">
            <div
              className="progress-fill"
              style={{ width: `${conversionProgress}%` }}
            />
          </div>
          <span className="progress-text">转换中...</span>
        </div>
      )}

      {isProcessingImages && imageProcessingProgress > 0 && (
        <div className="toolbar-progress">
          <div className="progress-bar">
            <div
              className="progress-fill"
              style={{ width: `${imageProcessingProgress}%` }}
            />
          </div>
          <span className="progress-text">
            处理中 {imageProcessingProgress}% | 成功 {imageProcessingSuccess} 失败 {imageProcessingFailed} 复用 {imageProcessingSkipped}
            {imageProcessingTotal ? ` | 待处理 ${pendingImages}/${imageProcessingTotal}` : ''}
            {currentChapterName ? ` | 文件 ${currentChapterName}` : ''}
            {imageProcessingCurrentImageUrl ? ` | 图片 ${imageProcessingCurrentImageUrl}` : ''}
          </span>
        </div>
      )}
    </div>
  );
};
