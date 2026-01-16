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

        const result = await invoke<ImportResult>('import_epub_command', {
          filePath: selected,
        });

        if (result.success && result.epubInfo) {
          addEpub(result.epubInfo);
        } else {
          alert(result.error || '导入失败');
        }
      }
    } catch (error) {
      console.error('Failed to import EPUB:', error);
      alert(`导入失败: ${error}`);
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
  epubInfo?: EpubInfo;
  error?: string;
}

interface EpubInfo {
  id: string;
  name: string;
  path: string;
  loaded_at: number;
}
