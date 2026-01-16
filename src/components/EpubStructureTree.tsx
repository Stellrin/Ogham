import React from 'react';
import { useEpubStore } from '../store/epubStore';

export const EpubStructureTree: React.FC = () => {
  const { epubs, selectedEpubId } = useEpubStore();

  const selectedEpub = epubs.find((epub) => epub.id === selectedEpubId);

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
          <p>结构尚未加载</p>
          <p className="hint">{selectedEpub.name}</p>
        </div>
      </div>
    );
  }

  const structure = selectedEpub.structure;

  return (
    <div className="structure-tree">
      <div className="structure-header">
        <h3>EPUB 结构</h3>
        <span className="epub-filename">{selectedEpub.name}</span>
      </div>
      <div className="structure-content">
        <div className="tree-node">
          <span className="tree-icon">📁</span>
          <span className="tree-label">OEBPS/</span>
        </div>

        <div className="tree-children">
          {/* content.opf */}
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
          <div className="tree-node">
            <span className="tree-icon">📁</span>
            <span className="tree-label">Text/</span>
          </div>
          <div className="tree-children">
            {structure.chapters.map((chapter) => (
              <div key={chapter.id} className="tree-node tree-leaf">
                <span className="tree-icon">📖</span>
                <span className="tree-label file-name">{chapter.name}</span>
                {chapter.name === 'cover.xhtml' && (
                  <span className="tree-badge special">封面</span>
                )}
              </div>
            ))}
          </div>

          {/* Styles/ */}
          {structure.styles.length > 0 && (
            <>
              <div className="tree-node">
                <span className="tree-icon">📁</span>
                <span className="tree-label">Styles/</span>
              </div>
              <div className="tree-children">
                {structure.styles.map((style, index) => (
                  <div key={index} className="tree-node tree-leaf">
                    <span className="tree-icon">🎨</span>
                    <span className="tree-label file-name">{style}</span>
                  </div>
                ))}
              </div>
            </>
          )}

          {/* Images/ */}
          {structure.images.length > 0 && (
            <>
              <div className="tree-node">
                <span className="tree-icon">📁</span>
                <span className="tree-label">Images/</span>
              </div>
              <div className="tree-children">
                {structure.images.map((image, index) => (
                  <div key={index} className="tree-node tree-leaf">
                    <span className="tree-icon">🖼️</span>
                    <span className="tree-label file-name">{image}</span>
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
};
