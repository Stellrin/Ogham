import React, { useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { EpubFile, RefactoredEpubStructure, StandardEpubStructure, useEpubStore } from '../store/epubStore';

export const ImportButton: React.FC = () => {
  const addEpub = useEpubStore((state) => state.addEpub);
  const selectEpub = useEpubStore((state) => state.selectEpub);
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
          const result = await invoke<BackendEpubInfo>('import_epub_command', {
            filePath: selected,
          });

          const importedEpub = normalizeImportedEpub(result);
          addEpub(importedEpub);
          await selectEpub(importedEpub.id);
          addNotification({
            kind: 'success',
            title: '导入完成',
            message: importedEpub.name,
          });
        } catch (invokeError) {
          addNotification({
            kind: 'error',
            title: '调用后端失败',
            message: getErrorMessage(invokeError),
          });
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

interface BackendEpubInfo {
  id: string;
  name: string;
  path: string;
  loaded_at: number;
  loadedAt?: number;
  epub_id?: string;
  epubId?: string;
  refactored_structure?: BackendRefactoredEpubResult;
  refactoredStructure?: RefactoredEpubStructure;
}

interface BackendRefactoredEpubResult {
  epub_id: string;
  metadata: RefactoredEpubStructure['metadata'];
  structure: StandardEpubStructure;
  storage_path: string;
}

function normalizeImportedEpub(info: BackendEpubInfo): EpubFile {
  const backendRefactored = info.refactoredStructure || info.refactored_structure;
  const refactoredStructure = normalizeRefactoredStructure(backendRefactored);
  const epubId = info.epubId || info.epub_id || refactoredStructure?.epubId || info.id;

  return {
    id: info.id,
    name: info.name,
    path: info.path,
    loadedAt: info.loadedAt || info.loaded_at,
    epubId,
    refactoredStructure,
  };
}

function normalizeRefactoredStructure(
  refactored?: BackendRefactoredEpubResult | RefactoredEpubStructure
): RefactoredEpubStructure | undefined {
  if (!refactored) return undefined;

  if ('epubId' in refactored) {
    return refactored;
  }

  return {
    epubId: refactored.epub_id,
    metadata: refactored.metadata,
    structure: refactored.structure,
    storagePath: refactored.storage_path,
  };
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
