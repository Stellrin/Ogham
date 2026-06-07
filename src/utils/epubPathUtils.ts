import type { StandardChapter } from '../store/epubStore';

export type CombinedChapter = StandardChapter;

export function getChapterPath(chapter: CombinedChapter): string {
  return chapter.standard_path;
}

export function getChapterName(chapter: CombinedChapter): string {
  return chapter.original_filename || chapter.title || '';
}
