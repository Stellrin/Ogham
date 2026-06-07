import React from 'react';
import { useEpubStore, TocChapter } from '../store/epubStore';
import { ChevronDown, ChevronRight, FileText } from 'lucide-react';

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

  const getEntryFilePaths = (entry: TocChapter): string[] => {
    const paths = entry.filePaths?.length
      ? entry.filePaths
      : [entry.filePath].filter((path): path is string => Boolean(path));

    return Array.from(new Set(paths));
  };

  const getEntryFileNames = (entry: TocChapter): string[] => {
    const fileNames = entry.fileNames?.length
      ? entry.fileNames
      : [entry.fileName].filter((name): name is string => Boolean(name));

    return fileNames.length > 0 ? fileNames : getEntryFilePaths(entry);
  };

  const openChapterPath = (chapterPath: string, order: number, anchor?: string | null) => {
    if (chapterPath) {
      setReaderState({
        currentChapterPath: chapterPath,
        currentChapterIndex: order,
        scrollPosition: 0,
        viewingImagePath: null,
        viewingImageData: null,
        pendingAnchor: anchor || null,
      });
    }
  };

  const handleEntryClick = (entry: TocChapter) => {
    const chapterPath = getEntryFilePaths(entry)[0];
    if (chapterPath) {
      openChapterPath(chapterPath, entry.order);
    }
  };

  const renderEntry = (entry: TocChapter, depth: number = 0) => {
    const isExpanded = expandedTocIds.has(entry.id);
    const filePaths = getEntryFilePaths(entry);
    const fileNames = getEntryFileNames(entry);
    const chapterPath = filePaths[0] || '';
    const isActive = filePaths.some((filePath) => filePath === readerState.currentChapterPath);
    const hasChildren = entry.children.length > 0;
    const hasMultipleFiles = filePaths.length > 1;

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
              {isExpanded ? (
                <ChevronDown size={13} aria-hidden="true" />
              ) : (
                <ChevronRight size={13} aria-hidden="true" />
              )}
            </span>
          ) : (
            <span className="toc-expand-icon-placeholder" />
          )}

          {/* 章节标题 */}
          <FileText className="toc-item-icon" size={14} aria-hidden="true" />
          <span
            className="toc-label"
            onClick={() => handleEntryClick(entry)}
          >
            {entry.label}
          </span>

          {/* 文件路径 */}
          <span
            className="toc-file-path"
            title={hasMultipleFiles ? filePaths.join('\n') : chapterPath}
            onClick={() => handleEntryClick(entry)}
          >
            {hasMultipleFiles ? `${filePaths.length} 个文件` : fileNames[0] || chapterPath}
          </span>
        </div>

        {hasMultipleFiles && (
          <div className="toc-linked-files">
            {filePaths.map((filePath, index) => {
              const isFileActive = filePath === readerState.currentChapterPath;

              return (
                <button
                  key={filePath}
                  type="button"
                  className={`toc-linked-file ${isFileActive ? 'active' : ''}`}
                  title={filePath}
                  onClick={(event) => {
                    event.stopPropagation();
                    openChapterPath(filePath, entry.order, index === 0 ? entry.anchor : null);
                  }}
                >
                  <span className="toc-linked-file-index">{index + 1}</span>
                  <span className="toc-linked-file-name">{fileNames[index] || filePath}</span>
                </button>
              );
            })}
          </div>
        )}

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
