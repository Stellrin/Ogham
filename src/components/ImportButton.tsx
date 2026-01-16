import React, { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { useEpubStore } from '../store/epubStore';

export const ImportButton: React.FC = () => {
  const addEpub = useEpubStore((state) => state.addEpub);
  const [importing, setImporting] = useState(false);

  const handleImport = async () => {
    if (importing) return;

    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'EPUB',
            extensions: ['epub'],
          },
        ],
      });

      if (selected && typeof selected === 'string') {
        setImporting(true);

        try {
          const result = await invoke<ImportResult>('import_epub_command', {
            filePath: selected,
          });

          if (result.success && (result.epubInfo || result.epub_info)) {
            addEpub((result.epubInfo || result.epub_info) as any);
          } else {
            alert(`导入失败: ${result.error || '未知错误'}`);
          }
        } catch (invokeError) {
          const errorMsg = invokeError instanceof Error ? invokeError.message : String(invokeError);
          alert(`调用后端失败: ${errorMsg}`);
        }
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      alert(`导入失败: ${errorMessage}`);
    } finally {
      setImporting(false);
    }
  };

  return (
    <button
      className="import-button"
      onClick={handleImport}
      disabled={importing}
    >
      <span className="import-icon">{importing ? '⏳' : '+'}</span>
      {importing ? '导入中...' : '导入 EPUB'}
    </button>
  );
};

interface ImportResult {
  success: boolean;
  epub_info?: EpubInfo;
  epubInfo?: EpubInfo;
  error?: string;
}

interface EpubInfo {
  id: string;
  name: string;
  path: string;
  loaded_at: number;
  loadedAt?: number;
}
