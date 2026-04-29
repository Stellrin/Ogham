import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { findChapterByHref, parseEpubHref } from '../utils/epubPathUtils';

export interface ChapterContent {
  html: string;
  resources: Record<string, ResourceData>;
}

export interface ResourceData {
  mimeType: string;
  data: string;
}

export interface EpubMetadata {
  title: string;
  author?: string;
  language?: string;
  identifier: string;
}

export interface EpubFile {
  id: string;
  name: string;
  path: string;
  structureError?: string;
  loadedAt: number;
  refactoredStructure?: RefactoredEpubStructure;
  epubId?: string;
}

export interface ReaderState {
  currentChapterIndex: number;
  currentChapterPath: string | null;
  scrollPosition: number;
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  viewingImagePath: string | null;
  viewingImageData: string | null;
  pendingAnchor: string | null;
}

export type AppNotificationKind = 'success' | 'error' | 'warning' | 'info';

export interface AppNotification {
  id: string;
  kind: AppNotificationKind;
  title: string;
  message?: string;
  details?: string;
  createdAt: number;
}

export interface AppNotificationInput {
  kind: AppNotificationKind;
  title: string;
  message?: string;
  details?: string;
  timeoutMs?: number;
}

// 导出章节类型供组件使用；导入后 UI 只面向标准化章节
export type CombinedChapter = StandardChapter;

// 重构系统相关接口
export interface RefactoredEpubStructure {
  epubId: string;
  metadata: EpubMetadata;
  structure: StandardEpubStructure;
  storagePath: string;
}

export interface StandardEpubStructure {
  chapters: StandardChapter[];
  styles: string[];
  images: string[];
  fonts: string[];
  navigation: NavigationEntry[];
}

export interface StandardChapter {
  id: string;
  original_filename: string;
  standard_path: string;
  title?: string;
  order: number;
  content?: ChapterContent;
}

export interface NavigationEntry {
  id: string;
  label: string;
  content_src: string;
  level: number;
  children: NavigationEntry[];
}

// 目录章节 - 支持嵌套层级
export interface TocChapter {
  id: string;
  label: string;
  contentSrc: string;
  filePath?: string;
  filePaths?: string[];
  level: number;
  order: number;
  children: TocChapter[];
  isExpanded?: boolean;
}

interface BackendTocChapter {
  id: string;
  label: string;
  content_src?: string;
  contentSrc?: string;
  file_path?: string;
  filePath?: string;
  file_paths?: string[];
  filePaths?: string[];
  level: number;
  order: number;
  children: BackendTocChapter[];
}

interface BackendTocOrder {
  id: string;
  label: string;
  content_src: string;
  children: BackendTocOrder[];
}

interface ConversionProgressEvent {
  task_id: string;
  file_path?: string | null;
  processed_files: number;
  total_files: number;
  files_converted: number;
  progress: number;
  stage: string;
  message: string;
}

interface ConversionResult {
  task_id: string;
  files_converted: number;
  total_files: number;
}

// 视图模式
export type ViewMode = 'file' | 'toc';

interface BackendRefactoredEpubResult {
  epub_id: string;
  metadata: EpubMetadata;
  structure: StandardEpubStructure;
  storage_path: string;
}

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

interface ImageProcessFailure {
  chapter_path: string;
  image_url: string;
  error: string;
}

interface ImageProcessProgressEvent {
  task_id: string;
  chapter_path: string;
  current_chapter_index: number;
  total_chapters: number;
  image_url?: string | null;
  processed_unique_images: number;
  total_unique_images: number;
  successful_images: number;
  failed_images: number;
  skipped_duplicates: number;
  stage: string;
  message: string;
}

interface ImageProcessResult {
  task_id: string;
  total_chapters: number;
  processed_chapters: number;
  detected_raw_matches: number;
  detected_unique_urls: number;
  successful_images: number;
  failed_images: number;
  skipped_duplicates: number;
  inserted_images: number;
  failures: ImageProcessFailure[];
}

interface EpubStore {
  epubs: EpubFile[];
  selectedEpubId: string | null;
  readerState: ReaderState;

  // 目录管理相关状态
  viewMode: ViewMode;
  tocEntries: TocChapter[];
  expandedTocIds: Set<string>;

  // 简繁转换相关状态
  isConverting: boolean;
  conversionProgress: number;

  // 图片处理相关状态
  isProcessingImages: boolean;
  imageProcessingProgress: number;
  imageProcessingTotal: number;
  imageProcessingCurrentChapter: string;
  imageProcessingCurrentImageUrl: string;
  imageProcessingSuccess: number;
  imageProcessingFailed: number;
  imageProcessingSkipped: number;
  imageProcessingProcessedUnique: number;
  imageProcessingTaskId: string | null;
  imageProcessingFailures: ImageProcessFailure[];

  // 应用内通知
  notifications: AppNotification[];
  addNotification: (notification: AppNotificationInput) => string;
  dismissNotification: (id: string) => void;
  clearNotifications: () => void;

  importEpubFromPath: (filePath: string) => Promise<EpubFile>;
  addEpub: (file: EpubFile) => void;
  removeEpub: (id: string) => void;
  selectEpub: (id: string) => Promise<void>;
  setStructureError: (id: string, error: string) => void;
  updateRefactoredStructure: (id: string, refactoredStructure: RefactoredEpubStructure) => void;
  clearRefactoredStructure: (id: string) => void;

  // 目录管理方法
  setViewMode: (mode: ViewMode) => Promise<void>;
  loadTocEntries: (epubId: string) => Promise<void>;
  updateTocEntryLabel: (entryId: string, newLabel: string) => Promise<void>;
  updateTocEntryFile: (entryId: string, newContentSrc: string) => Promise<void>;
  reorderTocEntries: (newOrder: TocChapter[]) => Promise<void>;
  toggleTocExpanded: (entryId: string) => void;

  // 简繁转换方法
  convertSimplifiedTraditional: (mode: 's2t' | 't2s') => Promise<void>;
  setConverting: (isConverting: boolean) => void;
  clearChapterCache: (id: string) => void;

  // 图片链接处理方法
  processAllImages: () => Promise<void>;

  loadImageContent: (imagePath: string) => Promise<void>;
  /** 重构 EPUB（解析原始文件，会生成新的 epub_id） */
  refactorEpub: (id: string) => Promise<void>;
  /** 从缓存重新加载（读取现有缓存，保持相同的 epub_id） */
  reloadEpubStructure: (id: string) => Promise<void>;
  loadRefactoredChapter: (epubId: string, chapterPath: string) => Promise<void>;
  exportEpub: (epubId: string) => Promise<string>;
  setReaderState: (state: Partial<ReaderState>) => void;
  navigateChapter: (direction: 'prev' | 'next') => void;
}

const defaultReaderState: ReaderState = {
  currentChapterIndex: 0,
  currentChapterPath: null,
  scrollPosition: 0,
  fontSize: 16,
  fontFamily: '"Georgia", serif',
  lineHeight: 1.6,
  viewingImagePath: null,
  viewingImageData: null,
  pendingAnchor: null,
};

const NOTIFICATION_LIMIT = 6;
const defaultViewMode: ViewMode = 'toc';

function createNotificationId(): string {
  return `notice-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function createTaskId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function getErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function getFileName(filePath: string): string {
  return filePath.split(/[\\/]/).pop() || filePath;
}

function normalizeFilePathForCompare(filePath: string): string {
  return filePath.replace(/\\/g, '/').toLowerCase();
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

function normalizeTocChapter(entry: BackendTocChapter): TocChapter {
  const filePaths = entry.filePaths || entry.file_paths;

  return {
    id: entry.id,
    label: entry.label,
    contentSrc: entry.contentSrc || entry.content_src || '',
    filePath: entry.filePath || entry.file_path,
    filePaths,
    level: entry.level,
    order: entry.order,
    children: (entry.children || []).map(normalizeTocChapter),
  };
}

function toBackendTocOrder(entries: TocChapter[]): BackendTocOrder[] {
  return entries.map((entry) => ({
    id: entry.id,
    label: entry.label,
    content_src: entry.contentSrc,
    children: toBackendTocOrder(entry.children),
  }));
}

interface FirstTocPage {
  href: string;
  order: number;
  ancestorIds: string[];
}

interface FirstPageSelection {
  readerState: Partial<ReaderState>;
  expandedTocIds: string[];
}

function getFirstTocPage(entries: TocChapter[], ancestorIds: string[] = []): FirstTocPage | null {
  for (const entry of entries) {
    const href = [entry.filePaths?.[0], entry.filePath, entry.contentSrc].find(
      (path): path is string => Boolean(path?.trim())
    );

    if (href) {
      return { href, order: entry.order, ancestorIds };
    }

    const childPage = getFirstTocPage(entry.children, [...ancestorIds, entry.id]);
    if (childPage) {
      return childPage;
    }
  }

  return null;
}

function getFirstNavigationPage(
  entries: NavigationEntry[],
  ancestorIds: string[] = [],
  orderRef = { current: 0 }
): FirstTocPage | null {
  for (const entry of entries) {
    const order = orderRef.current++;
    const href = entry.content_src?.trim();

    if (href) {
      return { href, order, ancestorIds };
    }

    const childPage = getFirstNavigationPage(entry.children, [...ancestorIds, entry.id], orderRef);
    if (childPage) {
      return childPage;
    }
  }

  return null;
}

function getFirstChapterByOrder(chapters: StandardChapter[]): StandardChapter | undefined {
  return [...chapters].sort((a, b) => (a.order ?? 0) - (b.order ?? 0))[0];
}

function createFirstPageSelection(
  structure: StandardEpubStructure,
  tocEntries: TocChapter[]
): FirstPageSelection | null {
  const chapters = structure.chapters || [];
  if (chapters.length === 0) return null;

  const tocPage =
    getFirstTocPage(tocEntries) || getFirstNavigationPage(structure.navigation || []);
  if (tocPage) {
    const chapter = findChapterByHref(tocPage.href, chapters);
    if (chapter) {
      const { anchor } = parseEpubHref(tocPage.href);
      return {
        readerState: {
          currentChapterIndex: chapter.order ?? tocPage.order,
          currentChapterPath: chapter.standard_path,
          scrollPosition: 0,
          viewingImagePath: null,
          viewingImageData: null,
          pendingAnchor: anchor || null,
        },
        expandedTocIds: tocPage.ancestorIds,
      };
    }
  }

  const firstChapter = getFirstChapterByOrder(chapters);
  if (!firstChapter) return null;

  return {
    readerState: {
      currentChapterIndex: firstChapter.order ?? 0,
      currentChapterPath: firstChapter.standard_path,
      scrollPosition: 0,
      viewingImagePath: null,
      viewingImageData: null,
      pendingAnchor: null,
    },
    expandedTocIds: [],
  };
}

function isReaderWaitingForFirstPage(readerState: ReaderState): boolean {
  return !readerState.currentChapterPath && !readerState.viewingImagePath;
}

export const useEpubStore = create<EpubStore>((set, get) => ({
  epubs: [],
  selectedEpubId: null,
  readerState: defaultReaderState,
  viewMode: defaultViewMode,
  tocEntries: [],
  expandedTocIds: new Set<string>(),
  isConverting: false,
  conversionProgress: 0,
  isProcessingImages: false,
  imageProcessingProgress: 0,
  imageProcessingTotal: 0,
  imageProcessingCurrentChapter: '',
  imageProcessingCurrentImageUrl: '',
  imageProcessingSuccess: 0,
  imageProcessingFailed: 0,
  imageProcessingSkipped: 0,
  imageProcessingProcessedUnique: 0,
  imageProcessingTaskId: null,
  imageProcessingFailures: [],
  notifications: [],

  addNotification: (notification) => {
    const id = createNotificationId();
    const item: AppNotification = {
      id,
      kind: notification.kind,
      title: notification.title,
      message: notification.message,
      details: notification.details,
      createdAt: Date.now(),
    };

    set((state) => ({
      notifications: [item, ...state.notifications].slice(0, NOTIFICATION_LIMIT),
    }));

    const timeoutMs = notification.timeoutMs ?? (notification.kind === 'error' ? 0 : 4800);
    if (timeoutMs > 0) {
      globalThis.setTimeout(() => {
        get().dismissNotification(id);
      }, timeoutMs);
    }

    return id;
  },

  dismissNotification: (id) =>
    set((state) => ({
      notifications: state.notifications.filter((notification) => notification.id !== id),
    })),

  clearNotifications: () => set({ notifications: [] }),

  importEpubFromPath: async (filePath: string) => {
    const normalizedPath = normalizeFilePathForCompare(filePath);
    const existing = get().epubs.find(
      (epub) => normalizeFilePathForCompare(epub.path) === normalizedPath
    );

    if (existing) {
      await get().selectEpub(existing.id);
      get().addNotification({
        kind: 'info',
        title: 'EPUB 已在列表中',
        message: existing.name,
      });
      return existing;
    }

    try {
      const result = await invoke<BackendEpubInfo>('import_epub_command', {
        filePath,
      });

      const importedEpub = normalizeImportedEpub(result);
      get().addEpub(importedEpub);
      await get().selectEpub(importedEpub.id);
      get().addNotification({
        kind: 'success',
        title: '导入完成',
        message: importedEpub.name,
      });

      return importedEpub;
    } catch (error) {
      get().addNotification({
        kind: 'error',
        title: 'EPUB 打开失败',
        message: getFileName(filePath),
        details: getErrorMessage(error),
        timeoutMs: 0,
      });
      throw error;
    }
  },

  addEpub: (file) =>
    set((state) => {
      return {
        epubs: [...state.epubs, file],
      };
    }),

  removeEpub: (id) =>
    set((state) => ({
      epubs: state.epubs.filter((epub) => epub.id !== id),
      selectedEpubId: state.selectedEpubId === id ? null : state.selectedEpubId,
      tocEntries: state.selectedEpubId === id ? [] : state.tocEntries,
    })),

  selectEpub: async (id: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === id);

    set(() => ({
      selectedEpubId: id,
      readerState: defaultReaderState,
      viewMode: defaultViewMode,
      tocEntries: [],
      expandedTocIds: new Set<string>(),
    }));

    // 选择 EPUB 后确保只使用标准化后的管理目录
    if (epub && !epub.refactoredStructure) {
      await get().refactorEpub(id);
    } else {
      const managedEpubId = epub?.refactoredStructure?.epubId || epub?.epubId;
      if (managedEpubId) {
        await get().loadTocEntries(managedEpubId);
      }
    }
  },

  setStructureError: (id, error) =>
    set((state) => ({
      epubs: state.epubs.map((epub) =>
        epub.id === id ? { ...epub, structureError: error } : epub
      ),
    })),

  updateRefactoredStructure: (id, refactoredStructure) =>
    set((state) => ({
      epubs: state.epubs.map((epub) =>
        epub.id === id
          ? {
              ...epub,
              refactoredStructure,
              epubId: refactoredStructure.epubId,
              structureError: undefined,
            }
          : epub
      ),
    })),

  clearRefactoredStructure: (id) =>
    set((state) => {
      const isSelected = state.selectedEpubId === id;
      return {
        epubs: state.epubs.map((epub) =>
          epub.id === id
            ? {
                ...epub,
                refactoredStructure: undefined,
                epubId: undefined,
              }
            : epub
        ),
        tocEntries: isSelected ? [] : state.tocEntries,
        expandedTocIds: isSelected ? new Set<string>() : state.expandedTocIds,
      };
    }),

  loadImageContent: async (imagePath: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure?.epubId) return;

    try {
      const imageData = await invoke<string>('get_image_from_refactored_command', {
        epubId: epub.refactoredStructure.epubId,
        imagePath,
      });
      get().setReaderState({
        viewingImagePath: imagePath,
        viewingImageData: imageData,
      });
    } catch (error) {
      get().addNotification({
        kind: 'error',
        title: '图片加载失败',
        message: getErrorMessage(error),
      });
    }
  },

  setReaderState: (newState: Partial<ReaderState>) =>
    set((state) => ({
      readerState: { ...state.readerState, ...newState },
    })),

  navigateChapter: (direction: 'prev' | 'next') => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);

    const chapters = epub?.refactoredStructure?.structure.chapters;
    if (!chapters || chapters.length === 0) return;

    const currentPath = state.readerState.currentChapterPath;

    // 通过当前路径查找序号，避免 index 与 TOC order 不一致
    const foundIndex = chapters.findIndex((chapter) => chapter.standard_path === currentPath);
    const baseIndex = foundIndex >= 0 ? foundIndex : state.readerState.currentChapterIndex;

    let targetIndex: number;
    if (direction === 'prev') {
      targetIndex = Math.max(0, baseIndex - 1);
    } else {
      targetIndex = Math.min(chapters.length - 1, baseIndex + 1);
    }

    if (targetIndex !== baseIndex) {
      const targetChapter = chapters[targetIndex];
      get().setReaderState({
        currentChapterIndex: targetIndex,
        currentChapterPath: targetChapter.standard_path,
        scrollPosition: 0,
      });
    }
  },

  /**
   * 重构 EPUB 文件
   *
   * ⚠️ 重要说明：
   * 此方法会重新解析**原始 EPUB 文件**，而不是读取缓存目录。
   * 适用于：首次加载 EPUB 或需要从原始文件重新生成标准结构的情况。
   *
   * 与 reloadEpubStructure 的区别：
   * - refactorEpub：解析原始 EPUB 文件，生成新的缓存和 epub_id
   * - reloadEpubStructure：读取现有缓存，使用相同的 epub_id
   */
  refactorEpub: async (id: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === id);
    if (!epub) return;

    try {
      const result = await invoke<BackendRefactoredEpubResult>('refactor_epub_command', {
        epubPath: epub.path,
      });

      const refactoredStructure: RefactoredEpubStructure = {
        epubId: result.epub_id,
        metadata: result.metadata,
        structure: result.structure,
        storagePath: result.storage_path,
      };

      get().updateRefactoredStructure(id, refactoredStructure);

      // 从管理目录重新读取完整结构，保证前端显示的是落盘后的最新状态
      await get().reloadEpubStructure(id);
    } catch (error) {
      const errorMessage = getErrorMessage(error);
      get().setStructureError(id, errorMessage);
      get().addNotification({
        kind: 'error',
        title: 'EPUB 标准化失败',
        message: epub.name,
        details: errorMessage,
      });
    }
  },

  /**
   * 从缓存目录重新加载 EPUB 结构
   *
   * ⚠️ 重要说明：
   * 此方法从**缓存目录**读取当前的文件结构，而不是重新解析原始 EPUB 文件。
   * 适用于：简繁转换、图片处理等操作后，需要刷新前端视图的情况。
   *
   * 与 refactorEpub 的区别：
   * - refactorEpub：解析原始 EPUB 文件，生成新的缓存和 epub_id
   * - reloadEpubStructure：读取现有缓存，使用相同的 epub_id
   */
  reloadEpubStructure: async (id: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === id);
    if (!epub?.refactoredStructure) return;

    try {
      const result = await invoke<BackendRefactoredEpubResult>('reload_epub_structure_command', {
        epubId: epub.refactoredStructure.epubId,
      });

      const refactoredStructure: RefactoredEpubStructure = {
        epubId: result.epub_id,
        metadata: result.metadata,
        structure: result.structure,
        storagePath: result.storage_path,
      };

      get().updateRefactoredStructure(id, refactoredStructure);

      // 重新加载目录数据
      await get().loadTocEntries(result.epub_id);
    } catch (error) {
      get().addNotification({
        kind: 'error',
        title: '刷新 EPUB 结构失败',
        message: epub.name,
        details: getErrorMessage(error),
      });
    }
  },

  loadRefactoredChapter: async (epubId: string, chapterPath: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub) return;

    try {
      const content = await invoke<ChapterContent>('get_chapter_from_refactored_command', {
        epubId,
        chapterPath,
      });

      set((state) => ({
        epubs: state.epubs.map((e) => {
          if (e.id === epub.id) {
            return {
              ...e,
              refactoredStructure: e.refactoredStructure
                ? {
                    ...e.refactoredStructure,
                    structure: {
                      ...e.refactoredStructure.structure,
                      chapters: e.refactoredStructure.structure.chapters.map((ch) =>
                        ch.standard_path === chapterPath ? { ...ch, content } : ch
                      ),
                    },
                  }
                : undefined,
            };
          }
          return e;
        }),
      }));
    } catch (error) {
      throw new Error(getErrorMessage(error));
    }
  },

  exportEpub: async (epubId: string) => {
    const state = get();
    // 首先通过 epubId 查找，如果找不到则通过 id 查找
    let epub = state.epubs.find((e) => e.epubId === epubId);
    if (!epub) {
      epub = state.epubs.find((e) => e.id === epubId);
    }
    if (!epub) {
      throw new Error('EPUB not found');
    }

    // 如果 EPUB 还没有重构，先重构它
    if (!epub.refactoredStructure) {
      await get().refactorEpub(epub.id);
    }

    // 获取更新后的 EPUB 信息
    const updatedEpub = get().epubs.find((e) => e.id === epub.id);
    if (!updatedEpub?.refactoredStructure) {
      throw new Error('EPUB refactoring failed');
    }

    return updatedEpub.refactoredStructure.epubId;
  },

  // 目录管理方法实现
  setViewMode: async (mode: ViewMode) => {
    // 切换到目录视图时，如果 EPUB 尚未重构则自动重构
    if (mode === 'toc') {
      const state = get();
      const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
      if (epub && !epub.refactoredStructure) {
        await get().refactorEpub(epub.id);
      }
    }
    set({ viewMode: mode });
  },

  loadTocEntries: async (epubId: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.epubId === epubId || e.id === epubId);
    if (!epub?.refactoredStructure) return;
    const isRequestForSelectedEpub = () => {
      const latestState = get();
      const selectedEpub = latestState.epubs.find((e) => e.id === latestState.selectedEpubId);
      return (
        selectedEpub?.id === epubId ||
        selectedEpub?.epubId === epubId ||
        selectedEpub?.refactoredStructure?.epubId === epubId
      );
    };

    try {
      const backendEntries = await invoke<BackendTocChapter[]>('load_toc_entries_command', {
        epubId,
      });
      const entries = backendEntries.map(normalizeTocChapter);

      if (isRequestForSelectedEpub()) {
        const latestState = get();
        const selectedEpub = latestState.epubs.find((e) => e.id === latestState.selectedEpubId);
        const firstPageSelection =
          selectedEpub?.refactoredStructure && isReaderWaitingForFirstPage(latestState.readerState)
            ? createFirstPageSelection(selectedEpub.refactoredStructure.structure, entries)
            : null;

        set({
          tocEntries: entries,
          expandedTocIds: firstPageSelection
            ? new Set([...latestState.expandedTocIds, ...firstPageSelection.expandedTocIds])
            : latestState.expandedTocIds,
          readerState: firstPageSelection
            ? { ...latestState.readerState, ...firstPageSelection.readerState }
            : latestState.readerState,
        });
      }
    } catch (error) {
      if (isRequestForSelectedEpub()) {
        const latestState = get();
        const selectedEpub = latestState.epubs.find((e) => e.id === latestState.selectedEpubId);
        const firstPageSelection =
          selectedEpub?.refactoredStructure && isReaderWaitingForFirstPage(latestState.readerState)
            ? createFirstPageSelection(selectedEpub.refactoredStructure.structure, [])
            : null;

        set({
          tocEntries: [],
          expandedTocIds: firstPageSelection
            ? new Set([...latestState.expandedTocIds, ...firstPageSelection.expandedTocIds])
            : latestState.expandedTocIds,
          readerState: firstPageSelection
            ? { ...latestState.readerState, ...firstPageSelection.readerState }
            : latestState.readerState,
        });
        get().addNotification({
          kind: 'warning',
          title: '目录加载失败',
          message: epub.name,
          details: getErrorMessage(error),
        });
      }
    }
  },

  updateTocEntryLabel: async (entryId: string, newLabel: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure) return;

    try {
      await invoke('update_toc_entry_command', {
        epubId: epub.refactoredStructure.epubId,
        entryId,
        newLabel,
        newContentSrc: null,
      });

      // 更新本地状态
      const updateLabel = (entries: TocChapter[]): TocChapter[] => {
        return entries.map((entry) => {
          if (entry.id === entryId) {
            return { ...entry, label: newLabel };
          }
          if (entry.children.length > 0) {
            return { ...entry, children: updateLabel(entry.children) };
          }
          return entry;
        });
      };

      set((state) => ({
        tocEntries: updateLabel(state.tocEntries),
      }));

      await get().reloadEpubStructure(epub.id);
    } catch (error) {
      get().addNotification({
        kind: 'error',
        title: '目录标题更新失败',
        message: epub.name,
        details: getErrorMessage(error),
      });
    }
  },

  updateTocEntryFile: async (entryId: string, newContentSrc: string) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure) return;

    try {
      await invoke('update_toc_entry_command', {
        epubId: epub.refactoredStructure.epubId,
        entryId,
        newLabel: null,
        newContentSrc,
      });

      // 更新本地状态
      const updateFile = (entries: TocChapter[]): TocChapter[] => {
        return entries.map((entry) => {
          if (entry.id === entryId) {
            return { ...entry, contentSrc: newContentSrc };
          }
          if (entry.children.length > 0) {
            return { ...entry, children: updateFile(entry.children) };
          }
          return entry;
        });
      };

      set((state) => ({
        tocEntries: updateFile(state.tocEntries),
      }));

      await get().reloadEpubStructure(epub.id);
    } catch (error) {
      get().addNotification({
        kind: 'error',
        title: '目录文件关联更新失败',
        message: epub.name,
        details: getErrorMessage(error),
      });
    }
  },

  reorderTocEntries: async (newOrder: TocChapter[]) => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure) return;

    try {
      await invoke('update_toc_order_command', {
        epubId: epub.refactoredStructure.epubId,
        newOrder: toBackendTocOrder(newOrder),
      });

      set({ tocEntries: newOrder });
      await get().reloadEpubStructure(epub.id);
    } catch (error) {
      get().addNotification({
        kind: 'error',
        title: '目录顺序更新失败',
        message: epub.name,
        details: getErrorMessage(error),
      });
    }
  },

  toggleTocExpanded: (entryId: string) =>
    set((state) => {
      const newExpanded = new Set(state.expandedTocIds);
      if (newExpanded.has(entryId)) {
        newExpanded.delete(entryId);
      } else {
        newExpanded.add(entryId);
      }
      return { expandedTocIds: newExpanded };
    }),

  setConverting: (isConverting: boolean) =>
    set({ isConverting }),

  clearChapterCache: (id: string) =>
    set((state) => ({
      epubs: state.epubs.map((epub) =>
        epub.id === id && epub.refactoredStructure
          ? {
              ...epub,
              refactoredStructure: {
                ...epub.refactoredStructure,
                structure: {
                  ...epub.refactoredStructure.structure,
                  chapters: epub.refactoredStructure.structure.chapters.map((ch) => ({
                    ...ch,
                    content: undefined,
                  })),
                },
              },
            }
          : epub
      ),
    })),

  convertSimplifiedTraditional: async (mode: 's2t' | 't2s') => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure) {
      throw new Error('EPUB not found or not refactored');
    }

    const epubId = epub.refactoredStructure.epubId;
    const taskId = createTaskId('conversion');
    let unlisten: (() => void) | null = null;

    try {
      set({ isConverting: true, conversionProgress: 0 });

      unlisten = await listen<ConversionProgressEvent>('conversion_progress', (event) => {
        const payload = event.payload;
        if (payload.task_id !== taskId) {
          return;
        }

        set({
          conversionProgress: Math.min(payload.progress || 0, 100),
        });
      });

      const result = await invoke<ConversionResult>(
        'convert_simplified_traditional_command',
        {
          epubId,
          mode,
          taskId,
        }
      );

      if (unlisten) {
        unlisten();
        unlisten = null;
      }

      // 显示进度 100%
      set({ conversionProgress: 100 });

      // 清除章节缓存，确保下次加载获取转换后的最新内容
      get().clearChapterCache(epub.id);

      // 先清理状态，避免阻塞 UI 更新
      set({ isConverting: false, conversionProgress: 0 });

      // 重新加载 EPUB 结构以更新文件视图（直接从缓存读取）
      await get().reloadEpubStructure(epub.id);

      get().addNotification({
        kind: 'success',
        title: '简繁转换完成',
        message: `共转换 ${result.files_converted}/${result.total_files} 个文件`,
      });
    } catch (error) {
      // 清理状态
      if (unlisten) {
        unlisten();
      }
      set({ isConverting: false, conversionProgress: 0 });

      const errorMessage = getErrorMessage(error);
      get().addNotification({
        kind: 'error',
        title: '简繁转换失败',
        message: epub.name,
        details: errorMessage,
      });
      throw error;
    }
  },

  // 处理整本 EPUB 中所有章节的图片链接
  processAllImages: async () => {
    const state = get();
    const epub = state.epubs.find((e) => e.id === state.selectedEpubId);
    if (!epub?.refactoredStructure) {
      throw new Error('EPUB not found or not refactored');
    }

    const epubId = epub.refactoredStructure.epubId;
    const taskId = createTaskId('image-process');
    let unlisten: (() => void) | null = null;

    try {
      set({
        isProcessingImages: true,
        imageProcessingProgress: 0,
        imageProcessingTotal: 0,
        imageProcessingCurrentChapter: '',
        imageProcessingCurrentImageUrl: '',
        imageProcessingSuccess: 0,
        imageProcessingFailed: 0,
        imageProcessingSkipped: 0,
        imageProcessingProcessedUnique: 0,
        imageProcessingTaskId: taskId,
        imageProcessingFailures: [],
      });

      unlisten = await listen<ImageProcessProgressEvent>('image_process_progress', (event) => {
        const payload = event.payload;

        set((currentState) => {
          if (payload.task_id !== taskId) {
            return currentState;
          }

          const total = payload.total_unique_images || 0;
          const processed = payload.processed_unique_images || 0;
          const totalChapters = payload.total_chapters || 0;
          const currentChapter = payload.current_chapter_index || 0;
          const imageProgress = total > 0 ? Math.round((processed / total) * 100) : 0;
          const chapterProgress = totalChapters > 0
            ? Math.round((currentChapter / totalChapters) * 100)
            : 0;
          const progress = total > 0 ? imageProgress : chapterProgress;

          return {
            imageProcessingTaskId: payload.task_id,
            imageProcessingTotal: total,
            imageProcessingProcessedUnique: processed,
            imageProcessingProgress: Math.min(progress, 100),
            imageProcessingCurrentChapter: payload.chapter_path || '',
            imageProcessingCurrentImageUrl: payload.image_url || '',
            imageProcessingSuccess: payload.successful_images || 0,
            imageProcessingFailed: payload.failed_images || 0,
            imageProcessingSkipped: payload.skipped_duplicates || 0,
          };
        });
      });

      const result = await invoke<ImageProcessResult>('process_all_images_command', {
        epubId,
        taskId,
      });

      if (unlisten) {
        unlisten();
        unlisten = null;
      }

      set({
        imageProcessingTaskId: result.task_id,
        imageProcessingTotal: result.detected_unique_urls,
        imageProcessingProcessedUnique: result.detected_unique_urls,
        imageProcessingSuccess: result.successful_images,
        imageProcessingFailed: result.failed_images,
        imageProcessingSkipped: result.skipped_duplicates,
        imageProcessingFailures: result.failures,
        imageProcessingProgress: 100,
      });

      // 清除章节缓存，确保下次加载获取最新内容
      const didUpdateContent = result.inserted_images > 0;
      if (didUpdateContent) {
        get().clearChapterCache(epub.id);

        // 重新加载 EPUB 结构以更新文件视图（包括新增图片，直接从缓存读取）
        await get().reloadEpubStructure(epub.id);
      }

      // 先清理状态，避免阻塞 UI 更新
      set({
        isProcessingImages: false,
        imageProcessingProgress: 0,
        imageProcessingCurrentChapter: '',
        imageProcessingCurrentImageUrl: '',
      });

      const summary = `章节 ${result.processed_chapters}/${result.total_chapters}，` +
        `匹配 ${result.detected_raw_matches} 条，去重后 ${result.detected_unique_urls} 张，` +
        `成功 ${result.successful_images}，失败 ${result.failed_images}，复用 ${result.skipped_duplicates}`;
      const failureDetails = result.failures
        .slice(0, 8)
        .map((failure, index) => `${index + 1}. ${failure.chapter_path} | ${failure.image_url} | ${failure.error}`)
        .join('\n');

      get().addNotification({
        kind: result.failed_images > 0 ? 'warning' : result.detected_unique_urls === 0 ? 'info' : 'success',
        title: result.failed_images > 0
          ? '图片处理完成，但有失败项'
          : result.detected_unique_urls === 0
            ? '没有发现待处理图片链接'
            : '图片处理完成',
        message: summary,
        details: failureDetails || undefined,
        timeoutMs: result.failed_images > 0 ? 0 : undefined,
      });
    } catch (error) {
      // 清理状态
      if (unlisten) {
        unlisten();
      }
      set({
        isProcessingImages: false,
        imageProcessingProgress: 0,
        imageProcessingCurrentChapter: '',
        imageProcessingCurrentImageUrl: '',
      });

      const errorMessage = getErrorMessage(error);
      get().addNotification({
        kind: 'error',
        title: '处理图片失败',
        message: epub.name,
        details: errorMessage,
      });
      throw error;
    }
  },
}));
