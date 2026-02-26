import React, { useState, useEffect } from 'react';
import { useEpubStore, CombinedChapter, TocChapter, NavigationEntry } from '../store/epubStore';
import { getChapterPath, getChapterName } from '../utils/epubPathUtils';
import { TocTreeView } from './TocTreeView';

// 将 NavigationEntry 转换为 TocChapter 格式
const convertNavigationToToc = (navigation: NavigationEntry[], chapters: CombinedChapter[]): TocChapter[] => {
  const flattenChapters = (items: NavigationEntry[], level: number, orderRef: { current: number }): TocChapter[] => {
    return items.map((item) => {
      const order = orderRef.current++;
      // 查找对应的章节文件路径
      const chapter = chapters.find((ch) => {
        const chPath = getChapterPath(ch);
        return chPath && (chPath.includes(item.content_src) || item.content_src.includes(chPath.split('/').pop() || ''));
      });
      const filePath = chapter ? getChapterPath(chapter) : undefined;

      return {
        id: item.id,
        label: item.label,
        contentSrc: item.content_src,
        filePath,
        level,
        order,
        children: flattenChapters(item.children, level + 1, orderRef),
      };
    });
  };

  const orderRef = { current: 0 };
  return flattenChapters(navigation, 0, orderRef);
};

// 将章节列表转换为 TocChapter 格式（当没有导航信息时使用）
const convertChaptersToToc = (chapters: CombinedChapter[]): TocChapter[] => {
  return chapters.map((chapter, index) => {
    // StandardChapter 有 title 字段，Chapter 有 name 字段
    const title = 'title' in chapter ? chapter.title : 'name' in chapter ? (chapter as any).name : undefined;
    return {
      id: `chapter-${index}`,
      label: title || getChapterName(chapter) || `Chapter ${index + 1}`,
      contentSrc: getChapterPath(chapter) || '',
      filePath: getChapterPath(chapter),
      level: 0,
      order: chapter.order ?? index,
      children: [],
    };
  });
};

export const EpubStructureTree: React.FC = () => {
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(['text']));

  const toggleSection = (section: string) => {
    setExpandedSections((prev) => {
      const next = new Set(prev);
      if (next.has(section)) {
        next.delete(section);
      } else {
        next.add(section);
      }
      return next;
    });
  };
  const {
    epubs,
    selectedEpubId,
    readerState,
    setReaderState,
    loadImageContent,
    viewMode,
    setViewMode,
    tocEntries,
  } = useEpubStore();

  const selectedEpub = epubs.find((epub) => epub.id === selectedEpubId);

  // 获取文件列表 - 优先使用重构后的结构
  const imageList: string[] = selectedEpub?.refactoredStructure?.structure.images ||
                               selectedEpub?.structure?.images ||
                               [];
  const styleList: string[] = selectedEpub?.refactoredStructure?.structure.styles ||
                                selectedEpub?.structure?.styles ||
                                [];

  // 优先使用重构后的结构，回退到原始结构
  const allChapters: CombinedChapter[] = selectedEpub?.refactoredStructure?.structure.chapters ||
                                          selectedEpub?.structure?.chapters ||
                                          [];

  // 自动展开 Text 文件夹当有章节被选中时
  useEffect(() => {
    if (readerState.currentChapterPath) {
      setExpandedSections((prev) => {
        const next = new Set(prev);
        next.add('text');
        return next;
      });
    }
  }, [readerState.currentChapterPath]);

  const handleChapterClick = (e: React.MouseEvent, chapter: CombinedChapter) => {
    e.stopPropagation();
    const chapterPath = getChapterPath(chapter);
    setReaderState({
      currentChapterIndex: chapter.order,
      currentChapterPath: chapterPath,
      scrollPosition: 0,
      viewingImagePath: null,
      viewingImageData: null,
    });
  };

  const handleImageClick = (e: React.MouseEvent, imagePath: string) => {
    e.stopPropagation();
    loadImageContent(imagePath);
  };

  if (!selectedEpub) {
    return (
      <div className="structure-tree">
        <div className="structure-empty">
          <p>请选择一个 EPUB 文件</p>
          <p className="hint">在左侧列表中选择以查看结构</p>
        </div>
      </div>
    );
  }

  if (!selectedEpub.structure) {
    return (
      <div className="structure-tree">
        <div className="structure-empty">
          {selectedEpub.structureError ? (
            <>
              <p className="error">加载失败</p>
              <p className="hint error-message">{selectedEpub.structureError}</p>
              <p className="hint">{selectedEpub.name}</p>
            </>
          ) : (
            <>
              <p>结构尚未加载</p>
              <p className="hint">{selectedEpub.name}</p>
            </>
          )}
        </div>
      </div>
    );
  }

  const structure = selectedEpub.structure;

  // 可折叠的文件夹节点组件
  const FolderNode: React.FC<{
    label: string;
    sectionId: string;
    children: React.ReactNode;
  }> = ({ label, sectionId, children }) => {
    const isExpanded = expandedSections.has(sectionId);
    return (
      <>
        <div
          className="tree-node tree-folder"
          onClick={() => toggleSection(sectionId)}
        >
          <span className="tree-icon">{isExpanded ? '📂' : '📁'}</span>
          <span className="tree-label">{label}</span>
        </div>
        {isExpanded && <div className="tree-children">{children}</div>}
      </>
    );
  };

  return (
    <div className="structure-tree">
      <div className="structure-header">
        <h3>EPUB 结构</h3>
        <span className="epub-filename">{selectedEpub.name}</span>
      </div>

      {/* 视图切换器 */}
      <div className="view-mode-switcher">
        <button
          className={`view-mode-btn ${viewMode === 'file' ? 'active' : ''}`}
          onClick={() => setViewMode('file')}
        >
          文件
        </button>
        <button
          className={`view-mode-btn ${viewMode === 'toc' ? 'active' : ''}`}
          onClick={() => setViewMode('toc')}
        >
          目录
        </button>
      </div>

      {/* 根据视图模式显示不同内容 */}
      {viewMode === 'toc' ? (
        <div className="toc-view-container">
          {selectedEpub?.refactoredStructure?.structure ? (
            <TocTreeView
              entries={
                // 优先使用 tocEntries（由 loadTocEntries 加载的最新数据），否则使用 navigation
                tocEntries && tocEntries.length > 0
                  ? tocEntries
                  : selectedEpub.refactoredStructure.structure.navigation?.length > 0
                    ? convertNavigationToToc(selectedEpub.refactoredStructure.structure.navigation, allChapters)
                    : convertChaptersToToc(allChapters)
              }
            />
          ) : selectedEpub?.structure?.chapters ? (
            // 未重构的 EPUB 显示原始目录信息
            <div className="toc-view-container">
              <div className="toc-info-box">
                <p>当前为原始 EPUB 结构</p>
                <p className="hint">重构后可使用完整的目录管理功能，包括编辑标签、调整顺序等</p>
              </div>
              <div className="original-toc-list">
                <h4>原始章节列表</h4>
                {selectedEpub.structure.chapters.map((chapter, index) => (
                  <div
                    key={chapter.id}
                    className={`toc-item ${readerState.currentChapterPath === chapter.path ? 'active' : ''}`}
                    onClick={() => {
                      setReaderState({
                        currentChapterIndex: index,
                        currentChapterPath: chapter.path,
                        scrollPosition: 0,
                      });
                    }}
                  >
                    <span className="toc-label">{chapter.name}</span>
                    <span className="toc-file-path">{chapter.path}</span>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <div className="structure-empty">
              <p>请先加载 EPUB 结构</p>
              <p className="hint">在左侧选择 EPUB 文件后查看目录</p>
            </div>
          )}
        </div>
      ) : (
      <div className="structure-content">
        <FolderNode label="OEBPS/" sectionId="oebps">
          <div className="tree-node tree-leaf">
            <span className="tree-icon">📄</span>
            <span className="tree-label file-name">content.opf</span>
            <span className="tree-badge">OPF</span>
          </div>

          {/* toc.ncx */}
          {structure.tocNcx && (
            <div className="tree-node tree-leaf">
              <span className="tree-icon">📄</span>
              <span className="tree-label file-name">toc.ncx</span>
              <span className="tree-badge special">NCX</span>
            </div>
          )}

          {/* nav.xhtml */}
          {structure.navXhtml && (
            <div className="tree-node tree-leaf">
              <span className="tree-icon">📄</span>
              <span className="tree-label file-name">nav.xhtml</span>
              <span className="tree-badge special">NAV</span>
            </div>
          )}

          {/* Text/ */}
          <FolderNode label="Text/" sectionId="text">
            {allChapters.map((chapter) => {
              const chapterPath = getChapterPath(chapter);
              const chapterName = getChapterName(chapter);
              const isActive = chapterPath === readerState.currentChapterPath;
              return (
                <div
                  key={chapter.id}
                  className={`tree-node tree-leaf chapter-item ${isActive ? 'active' : ''}`}
                  onClick={(e) => handleChapterClick(e, chapter)}
                >
                  <span className="tree-icon">📖</span>
                  <span className="tree-label file-name">{chapterName}</span>
                  {(chapterName === 'cover.xhtml' || chapterName === 'cover') && (
                    <span className="tree-badge special">封面</span>
                  )}
                </div>
              );
            })}
          </FolderNode>

          {/* Styles/ */}
          {styleList.length > 0 && (
            <FolderNode label="Styles/" sectionId="styles">
              {styleList.map((style, index) => {
                const fileName = style.split('/').pop() || style;
                return (
                  <div key={index} className="tree-node tree-leaf">
                    <span className="tree-icon">🎨</span>
                    <span className="tree-label file-name">{fileName}</span>
                  </div>
                );
              })}
            </FolderNode>
          )}

          {/* Images/ */}
          {imageList.length > 0 && (
            <FolderNode label="Images/" sectionId="images">
              {imageList.map((image, index) => {
                const fileName = image.split('/').pop() || image;
                const isViewing = readerState.viewingImagePath === image;
                return (
                  <div
                    key={index}
                    className={`tree-node tree-leaf image-item ${isViewing ? 'active' : ''}`}
                    onClick={(e) => handleImageClick(e, image)}
                  >
                    <span className="tree-icon">🖼️</span>
                    <span className="tree-label file-name">{fileName}</span>
                  </div>
                );
              })}
            </FolderNode>
          )}
        </FolderNode>
      </div>
      )}
    </div>
  );
};
