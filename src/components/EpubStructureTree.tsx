import React, { useState, useEffect } from 'react';
import { useEpubStore, CombinedChapter } from '../store/epubStore';
import { getChapterPath, getChapterName } from '../utils/epubPathUtils';
import { TocTreeView } from './TocTreeView';
import {
  BookText,
  ChevronDown,
  ChevronRight,
  FileCode2,
  Folder,
  FolderOpen,
  Image,
  Palette,
} from 'lucide-react';

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
    tocEntriesEpubId,
  } = useEpubStore();

  const selectedEpub = epubs.find((epub) => epub.id === selectedEpubId);
  const managedStructure = selectedEpub?.refactoredStructure?.structure;
  const selectedManagedEpubId =
    selectedEpub?.refactoredStructure?.epubId || selectedEpub?.epubId || selectedEpub?.id;

  const imageList: string[] = managedStructure?.images || [];
  const styleList: string[] = managedStructure?.styles || [];

  const allChapters: CombinedChapter[] = managedStructure?.chapters || [];

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
          <p className="hint">在左侧列表中选择以查看目录</p>
        </div>
      </div>
    );
  }

  if (!managedStructure) {
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
              <p>目录尚未加载</p>
              <p className="hint">{selectedEpub.name}</p>
            </>
          )}
        </div>
      </div>
    );
  }

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
          {isExpanded ? (
            <ChevronDown className="tree-chevron" size={13} aria-hidden="true" />
          ) : (
            <ChevronRight className="tree-chevron" size={13} aria-hidden="true" />
          )}
          {isExpanded ? (
            <FolderOpen className="tree-icon" size={15} aria-hidden="true" />
          ) : (
            <Folder className="tree-icon" size={15} aria-hidden="true" />
          )}
          <span className="tree-label">{label}</span>
        </div>
        {isExpanded && <div className="tree-children">{children}</div>}
      </>
    );
  };

  const tocTreeEntries =
    tocEntriesEpubId === selectedManagedEpubId ? tocEntries : [];

  return (
    <div className="structure-tree">
      <div className="structure-header">
        <h3>{viewMode === 'toc' ? '目录视图' : '文件结构'}</h3>
        <span className="epub-filename">{selectedEpub.name}</span>
      </div>

      {/* 视图切换器 */}
      <div className="view-mode-switcher">
        <button
          className={`view-mode-btn ${viewMode === 'toc' ? 'active' : ''}`}
          onClick={() => setViewMode('toc')}
          aria-pressed={viewMode === 'toc'}
        >
          目录
        </button>
        <button
          className={`view-mode-btn ${viewMode === 'file' ? 'active' : ''}`}
          onClick={() => setViewMode('file')}
          aria-pressed={viewMode === 'file'}
        >
          文件
        </button>
      </div>

      {/* 根据视图模式显示不同内容 */}
      {viewMode === 'toc' ? (
        <div className="toc-view-container">
          <TocTreeView entries={tocTreeEntries} />
        </div>
      ) : (
      <div className="structure-content">
        <FolderNode label="OEBPS/" sectionId="oebps">
          <div className="tree-node tree-leaf">
            <FileCode2 className="tree-icon" size={15} aria-hidden="true" />
            <span className="tree-label file-name">content.opf</span>
            <span className="tree-badge">OPF</span>
          </div>

          <div className="tree-node tree-leaf">
            <FileCode2 className="tree-icon" size={15} aria-hidden="true" />
            <span className="tree-label file-name">toc.ncx</span>
            <span className="tree-badge special">NCX</span>
          </div>

          <div className="tree-node tree-leaf">
            <FileCode2 className="tree-icon" size={15} aria-hidden="true" />
            <span className="tree-label file-name">nav.xhtml</span>
            <span className="tree-badge special">NAV</span>
          </div>

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
                  <BookText className="tree-icon" size={15} aria-hidden="true" />
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
                    <Palette className="tree-icon" size={15} aria-hidden="true" />
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
                    <Image className="tree-icon" size={15} aria-hidden="true" />
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
