import React from 'react';
import { useEpubStore } from '../store/epubStore';

export const EpubList: React.FC = () => {
  const { epubs, selectedEpubId, selectEpub, removeEpub } = useEpubStore();

  return (
    <div className="epub-list">
      <div className="epub-list-header">
        <h3>EPUB 列表</h3>
        <span className="epub-count">{epubs.length} 本</span>
      </div>
      <div className="epub-list-content">
        {epubs.length === 0 ? (
          <div className="epub-list-empty">
            <p>暂无 EPUB 文件</p>
            <p className="hint">点击下方按钮导入</p>
          </div>
        ) : (
          <ul className="epub-list-items">
            {epubs.map((epub) => (
              <li
                key={epub.id}
                className={`epub-item ${selectedEpubId === epub.id ? 'selected' : ''}`}
                onClick={() => selectEpub(epub.id)}
              >
                <div className="epub-item-info">
                  <span className="epub-name">{epub.name}</span>
                  <span className="epub-path">{epub.path}</span>
                </div>
                <button
                  className="epub-delete-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    removeEpub(epub.id);
                  }}
                  title="删除"
                >
                  ×
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
};
