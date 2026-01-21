import React, { useState, useEffect } from 'react';
import { useEpubStore, Chapter, StandardChapter, CombinedChapter } from '../store/epubStore';
import { getChapterPath, getChapterName } from '../utils/epubPathUtils';

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
  const { epubs, selectedEpubId, readerState, setReaderState, loadImageContent } = useEpubStore();

  const selectedEpub = epubs.find((epub) => epub.id === selectedEpubId);

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
          {structure.styles.length > 0 && (
            <FolderNode label="Styles/" sectionId="styles">
              {structure.styles.map((style, index) => {
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
          {structure.images.length > 0 && (
            <FolderNode label="Images/" sectionId="images">
              {structure.images.map((image, index) => {
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
    </div>
  );
};
