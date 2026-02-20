import React, { useState } from 'react';
import { useEpubStore, TocChapter } from '../store/epubStore';

interface TocTreeViewProps {
  entries: TocChapter[];
  onReorder?: (newOrder: TocChapter[]) => void;
}

export const TocTreeView: React.FC<TocTreeViewProps> = ({ entries, onReorder }) => {
  const {
    setReaderState,
    readerState,
    expandedTocIds,
    toggleTocExpanded,
    updateTocEntryLabel,
    updateTocEntryFile,
  } = useEpubStore();

  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState('');
  const [editType, setEditType] = useState<'label' | 'file'>('label');
  const [draggedId, setDraggedId] = useState<string | null>(null);

  const handleEntryClick = (entry: TocChapter) => {
    if (entry.filePath) {
      setReaderState({
        currentChapterPath: entry.filePath,
        currentChapterIndex: entry.order,
        scrollPosition: 0,
        viewingImagePath: null,
        viewingImageData: null,
      });
    }
  };

  const handleEditStart = (entry: TocChapter, type: 'label' | 'file') => {
    setEditingId(entry.id);
    setEditValue(type === 'label' ? entry.label : entry.contentSrc);
    setEditType(type);
  };

  const handleEditSave = async () => {
    if (!editingId) return;

    if (editType === 'label') {
      await updateTocEntryLabel(editingId, editValue);
    } else {
      await updateTocEntryFile(editingId, editValue);
    }

    setEditingId(null);
    setEditValue('');
  };

  const handleEditCancel = () => {
    setEditingId(null);
    setEditValue('');
  };

  const handleDragStart = (e: React.DragEvent, entryId: string) => {
    setDraggedId(entryId);
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', entryId);
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  };

  const handleDrop = (e: React.DragEvent, targetId: string) => {
    e.preventDefault();
    if (!draggedId || draggedId === targetId || !onReorder) return;

    // 简单的拖拽排序实现
    const findAndMove = (items: TocChapter[], targetId: string): TocChapter[] => {
      const result: TocChapter[] = [];
      let movedItem: TocChapter | null = null;

      for (const item of items) {
        if (item.id === draggedId) {
          movedItem = { ...item };
          continue;
        }

        if (item.id === targetId && movedItem) {
          result.push(movedItem);
          movedItem = null;
        }

        if (item.children.length > 0) {
          result.push({
            ...item,
            children: findAndMove(item.children, targetId),
          });
        } else {
          result.push(item);
        }
      }

      if (movedItem) {
        result.push(movedItem);
      }

      return result;
    };

    const newOrder = findAndMove([...entries], targetId);
    onReorder(newOrder);
    setDraggedId(null);
  };

  const renderEntry = (entry: TocChapter, depth: number = 0) => {
    const isExpanded = expandedTocIds.has(entry.id);
    const isActive = entry.filePath === readerState.currentChapterPath;
    const isEditing = editingId === entry.id;
    const hasChildren = entry.children.length > 0;

    return (
      <div key={entry.id} className="toc-entry" style={{ paddingLeft: `${depth * 16}px` }}>
        <div
          className={`toc-item ${isActive ? 'active' : ''} ${draggedId === entry.id ? 'dragging' : ''}`}
          draggable
          onDragStart={(e) => handleDragStart(e, entry.id)}
          onDragOver={handleDragOver}
          onDrop={(e) => handleDrop(e, entry.id)}
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
          {isEditing && editType === 'label' ? (
            <input
              type="text"
              className="toc-edit-input"
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              onBlur={handleEditSave}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleEditSave();
                if (e.key === 'Escape') handleEditCancel();
              }}
              autoFocus
            />
          ) : (
            <span
              className="toc-label"
              onClick={() => handleEntryClick(entry)}
              onDoubleClick={() => handleEditStart(entry, 'label')}
              title="双击编辑标题"
            >
              {entry.label}
            </span>
          )}

          {/* 文件路径 */}
          <span
            className="toc-file-path"
            title={entry.contentSrc}
            onClick={() => handleEntryClick(entry)}
          >
            {entry.filePath?.split('/').pop() || entry.contentSrc}
          </span>

          {/* 编辑按钮 */}
          <button
            className="toc-edit-btn"
            onClick={() => handleEditStart(entry, 'label')}
            title="编辑标题"
          >
            ✏️
          </button>
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
