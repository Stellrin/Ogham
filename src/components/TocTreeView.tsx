import React from 'react';
import { useEpubStore, TocChapter } from '../store/epubStore';

interface TocTreeViewProps {
  entries: TocChapter[];
}

export const TocTreeView: React.FC<TocTreeViewProps> = ({ entries }) => {
  const {
    setReaderState,
    readerState,
    expandedTocIds,
    toggleTocExpanded,
  } = useEpubStore();

  const handleEntryClick = (entry: TocChapter) => {
    // 优先使用 filePath，回退到 contentSrc
    const chapterPath = entry.filePath || entry.contentSrc;
    if (chapterPath) {
      setReaderState({
        currentChapterPath: chapterPath,
        currentChapterIndex: entry.order,
        scrollPosition: 0,
        viewingImagePath: null,
        viewingImageData: null,
      });
    }
  };

  const renderEntry = (entry: TocChapter, depth: number = 0) => {
    const isExpanded = expandedTocIds.has(entry.id);
    const chapterPath = entry.filePath || entry.contentSrc;
    const isActive = chapterPath === readerState.currentChapterPath;
    const hasChildren = entry.children.length > 0;

    return (
      <div key={entry.id} className="toc-entry" style={{ paddingLeft: `${depth * 16}px` }}>
        <div
          className={`toc-item ${isActive ? 'active' : ''}`}
        >
          {/* 展开/折叠按钮 */}
          {hasChildren ? (
            <span
              className="toc-expand-icon"
              onClick={() => toggleTocExpanded(entry.id)}
            >
              {isExpanded ? '▼' : '▶'}
            </span>
          ) : (
            <span className="toc-expand-icon-placeholder" />
          )}

          {/* 章节标题 */}
          <span
            className="toc-label"
            onClick={() => handleEntryClick(entry)}
          >
            {entry.label}
          </span>

          {/* 文件路径 */}
          <span
            className="toc-file-path"
            title={entry.contentSrc}
            onClick={() => handleEntryClick(entry)}
          >
            {entry.filePath?.split('/').pop() || entry.contentSrc}
          </span>
        </div>

        {/* 子目录 */}
        {hasChildren && isExpanded && (
          <div className="toc-children">
            {entry.children.map((child) => renderEntry(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  if (entries.length === 0) {
    return (
      <div className="toc-empty">
        <p>目录为空</p>
        <p className="hint">该 EPUB 没有目录信息</p>
      </div>
    );
  }

  return (
    <div className="toc-tree-view">
      {entries.map((entry) => renderEntry(entry, 0))}
    </div>
  );
};
