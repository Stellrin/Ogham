import React from 'react';
import { useEpubStore, TocChapter } from '../store/epubStore';
import { getChapterPath } from '../utils/epubPathUtils';

interface TocTreeViewProps {
  entries: TocChapter[];
}

export const TocTreeView: React.FC<TocTreeViewProps> = ({ entries }) => {
  const {
    epubs,
    selectedEpubId,
    setReaderState,
    readerState,
    expandedTocIds,
    toggleTocExpanded,
  } = useEpubStore();

  const selectedEpub = epubs.find((epub) => epub.id === selectedEpubId);
  const chapters = selectedEpub?.refactoredStructure?.structure.chapters || [];

  const normalizePath = (path: string | null | undefined): string => {
    if (!path) return '';

    let normalized = path.split('#')[0].replace(/\\/g, '/').replace(/\/+/g, '/');
    try {
      normalized = decodeURIComponent(normalized);
    } catch {
      // 保留原路径
    }

    if (normalized.startsWith('Text/')) {
      normalized = `OEBPS/${normalized}`;
    }

    return normalized.toLowerCase();
  };

  const findChapterByPath = (chapterPath: string) => {
    const normalizedTarget = normalizePath(chapterPath);
    return chapters.find((chapter) => normalizePath(getChapterPath(chapter)) === normalizedTarget);
  };

  const getEntryFilePaths = (entry: TocChapter): string[] => {
    const paths = entry.filePaths?.length
      ? entry.filePaths
      : [entry.filePath || entry.contentSrc].filter(Boolean);

    return Array.from(new Set(paths));
  };

  const getFileName = (filePath: string): string => {
    return filePath.split(/[\\/]/).pop() || filePath;
  };

  const openChapterPath = (chapterPath: string, fallbackOrder: number) => {
    const chapter = findChapterByPath(chapterPath);
    const standardPath = chapter ? getChapterPath(chapter) : chapterPath;

    if (standardPath) {
      setReaderState({
        currentChapterPath: standardPath,
        currentChapterIndex: chapter?.order ?? fallbackOrder,
        scrollPosition: 0,
        viewingImagePath: null,
        viewingImageData: null,
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
    const chapterPath = filePaths[0] || entry.filePath || entry.contentSrc;
    const isActive = filePaths.some(
      (filePath) => normalizePath(filePath) === normalizePath(readerState.currentChapterPath)
    );
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
            title={hasMultipleFiles ? filePaths.join('\n') : chapterPath}
            onClick={() => handleEntryClick(entry)}
          >
            {hasMultipleFiles ? `${filePaths.length} 个文件` : getFileName(chapterPath)}
          </span>
        </div>

        {hasMultipleFiles && (
          <div className="toc-linked-files">
            {filePaths.map((filePath, index) => {
              const isFileActive =
                normalizePath(filePath) === normalizePath(readerState.currentChapterPath);

              return (
                <button
                  key={filePath}
                  type="button"
                  className={`toc-linked-file ${isFileActive ? 'active' : ''}`}
                  title={filePath}
                  onClick={(event) => {
                    event.stopPropagation();
                    openChapterPath(filePath, entry.order);
                  }}
                >
                  <span className="toc-linked-file-index">{index + 1}</span>
                  <span className="toc-linked-file-name">{getFileName(filePath)}</span>
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
