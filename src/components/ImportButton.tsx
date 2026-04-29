import React, { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { useEpubStore } from '../store/epubStore';

export const ImportButton: React.FC = () => {
  const importEpubFromPath = useEpubStore((state) => state.importEpubFromPath);
  const addNotification = useEpubStore((state) => state.addNotification);
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
          await importEpubFromPath(selected);
        } catch {
          // importEpubFromPath 已经把错误展示到通知中心
        }
      }
    } catch (error) {
      addNotification({
        kind: 'error',
        title: '导入失败',
        message: getErrorMessage(error),
      });
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

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
