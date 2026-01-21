import React, { useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { useEpubStore } from '../store/epubStore';
import './ExportButton.css';

export const ExportButton: React.FC<{ epubId: string; epubName: string }> = ({
  epubId,
  epubName,
}) => {
  const { exportEpub, clearRefactoredStructure, loadEpubStructure, epubs } = useEpubStore();
  const [exporting, setExporting] = useState(false);

  const handleExport = async () => {
    if (exporting) return;

    try {
      setExporting(true);

      // 首先确保 EPUB 已经重构
      const refactoredEpubId = await exportEpub(epubId);

      // 生成默认的文件名
      const defaultName = epubName.replace('.epub', '') + '_refactored.epub';

      // 打开保存对话框
      const filePath = await save({
        defaultPath: defaultName,
        filters: [
          {
            name: 'EPUB',
            extensions: ['epub'],
          },
        ],
      });

      if (filePath && typeof filePath === 'string') {
        // 调用后端导出命令
        await invoke('export_epub_command', {
          epubId: refactoredEpubId,
          exportPath: filePath,
        });

        // 导出成功后，清除重构结构并重新加载文档结构
        const epub = epubs.find((e) => e.id === epubId || e.epubId === epubId);
        if (epub) {
          clearRefactoredStructure(epub.id);
          await loadEpubStructure(epub.id);
        }

        alert(`导出成功: ${filePath}`);
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      alert(`导出失败: ${errorMessage}`);
    } finally {
      setExporting(false);
    }
  };

  return (
    <button
      className="export-button"
      onClick={handleExport}
      disabled={exporting}
      title="导出重构后的 EPUB 文件"
    >
      <span className="export-icon">{exporting ? '⏳' : '📦'}</span>
      {exporting ? '导出中...' : '导出'}
    </button>
  );
};
