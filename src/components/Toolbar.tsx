import React, { useState, useRef, useEffect } from 'react';
import './Toolbar.css';
import { ChevronDown, FileImage, Loader2, Settings2 } from 'lucide-react';

export type ConversionMode = 's2t' | 't2s';

interface ToolbarProps {
  onConvert: (mode: ConversionMode) => void;
  isConverting: boolean;
  conversionProgress: number;
  onProcessAllImages: () => void;
  isProcessingImages: boolean;
  canRunTools?: boolean;
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
  canRunTools = true,
  imageProcessingProgress,
  imageProcessingTotal,
  imageProcessingCurrentChapter,
  imageProcessingCurrentImageUrl,
  imageProcessingSuccess = 0,
  imageProcessingFailed = 0,
  imageProcessingSkipped = 0,
  imageProcessingProcessedUnique = 0,
}) => {
  const [isToolsMenuOpen, setIsToolsMenuOpen] = useState(false);
  const toolsMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (toolsMenuRef.current && !toolsMenuRef.current.contains(event.target as Node)) {
        setIsToolsMenuOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  useEffect(() => {
    if (isConverting || isProcessingImages || !canRunTools) {
      setIsToolsMenuOpen(false);
    }
  }, [canRunTools, isConverting, isProcessingImages]);

  const isBusy = isConverting || isProcessingImages;
  const canUseTools = canRunTools && !isBusy;
  const menuButtonTitle = !canRunTools
    ? '请选择并加载 EPUB 后使用工具'
    : isBusy
      ? '当前任务完成后可继续操作'
      : '打开工具菜单';

  const handleSelect = (mode: ConversionMode) => {
    if (!canUseTools) return;

    setIsToolsMenuOpen(false);
    onConvert(mode);
  };

  const handleProcessImages = () => {
    if (!canUseTools) return;

    setIsToolsMenuOpen(false);
    onProcessAllImages();
  };

  const pendingImages = Math.max((imageProcessingTotal || 0) - imageProcessingProcessedUnique, 0);
  const currentChapterName = imageProcessingCurrentChapter
    ? imageProcessingCurrentChapter.split('/').pop() || imageProcessingCurrentChapter
    : '';
  const imageStatusTitle = [
    `成功 ${imageProcessingSuccess}`,
    `失败 ${imageProcessingFailed}`,
    `复用 ${imageProcessingSkipped}`,
    imageProcessingTotal ? `待处理 ${pendingImages}/${imageProcessingTotal}` : '',
    currentChapterName ? `文件 ${currentChapterName}` : '',
    imageProcessingCurrentImageUrl ? `图片 ${imageProcessingCurrentImageUrl}` : '',
  ].filter(Boolean).join(' | ');
  const progressValue = isConverting ? conversionProgress : imageProcessingProgress;
  const progressLabel = isConverting
    ? `简繁转换 ${conversionProgress}%`
    : `图片处理 ${imageProcessingProgress}%`;
  const progressDetail = isProcessingImages
    ? `成功 ${imageProcessingSuccess} / 失败 ${imageProcessingFailed} / 复用 ${imageProcessingSkipped}`
    : '正在更新 EPUB 内容';

  return (
    <div className="epub-toolbar" onKeyDown={(event) => {
      if (event.key === 'Escape') {
        setIsToolsMenuOpen(false);
      }
    }}>
      <div className="toolbar-actions" ref={toolsMenuRef}>
        <div className="toolbar-menu-container">
          <button
            type="button"
            className="toolbar-button toolbar-button-primary"
            onClick={() => setIsToolsMenuOpen((isOpen) => !isOpen)}
            disabled={!canUseTools}
            title={menuButtonTitle}
            aria-haspopup="menu"
            aria-expanded={isToolsMenuOpen}
          >
            <Settings2 className="toolbar-icon" size={15} aria-hidden="true" />
            <span className="toolbar-label">工具</span>
            <ChevronDown className="toolbar-caret" size={13} aria-hidden="true" />
          </button>

          {isToolsMenuOpen && (
            <div className="toolbar-menu" role="menu" aria-label="EPUB 工具">
              <div className="toolbar-menu-section">
                <div className="toolbar-menu-title">文字</div>
                <button
                  type="button"
                  className="toolbar-menu-item"
                  onClick={() => handleSelect('s2t')}
                  disabled={!canUseTools}
                  role="menuitem"
                >
                  <span className="toolbar-menu-icon">简</span>
                  <span className="toolbar-menu-copy">
                    <span className="toolbar-menu-label">简体转繁体</span>
                  </span>
                </button>
                <button
                  type="button"
                  className="toolbar-menu-item"
                  onClick={() => handleSelect('t2s')}
                  disabled={!canUseTools}
                  role="menuitem"
                >
                  <span className="toolbar-menu-icon">繁</span>
                  <span className="toolbar-menu-copy">
                    <span className="toolbar-menu-label">繁体转简体</span>
                  </span>
                </button>
              </div>

              <div className="toolbar-menu-divider" />

              <div className="toolbar-menu-section">
                <div className="toolbar-menu-title">资源</div>
                <button
                  type="button"
                  className="toolbar-menu-item"
                  onClick={handleProcessImages}
                  disabled={!canUseTools}
                  role="menuitem"
                >
                  <FileImage className="toolbar-menu-icon toolbar-menu-svg" size={15} aria-hidden="true" />
                  <span className="toolbar-menu-copy">
                    <span className="toolbar-menu-label">处理图片链接</span>
                  </span>
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      {isBusy && (
        <div className="toolbar-status" title={isProcessingImages ? imageStatusTitle : progressDetail}>
          <div className="toolbar-status-main">
            <Loader2 className="toolbar-status-dot is-spinning" size={13} aria-hidden="true" />
            <span className="toolbar-status-text">{progressLabel}</span>
          </div>
          <div className="toolbar-progress-bar" aria-hidden="true">
            <div
              className="toolbar-progress-fill"
              style={{ width: `${progressValue}%` }}
            />
          </div>
          <span className="toolbar-status-detail">
            {progressDetail}
          </span>
        </div>
      )}
    </div>
  );
};
