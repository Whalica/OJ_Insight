import { dirname, sep } from '@tauri-apps/api/path';
import { save } from '@tauri-apps/plugin-dialog';
import { openPath } from '@tauri-apps/plugin-opener';

import { api } from './api';

export async function saveTrainingFile(filename: string, data: string | number[], extension: 'json' | 'zip') {
  const storage = await api.storageInfo();
  const separator = sep();
  const slash = storage.exportDir.endsWith('/') || storage.exportDir.endsWith('\\') ? '' : separator;
  const path = await save({
    defaultPath: `${storage.exportDir}${slash}${filename}`,
    filters: [{ name: extension === 'zip' ? 'ZIP 压缩包' : 'OJ Insight 题单 JSON', extensions: [extension] }],
  });
  if (!path) return null;
  const bytes = typeof data === 'string' ? Array.from(new TextEncoder().encode(data)) : data;
  await api.writeExportFile(path, bytes);
  return { path, directory: storage.exportDir };
}

export function saveTrainingJson(filename: string, data: string) {
  return saveTrainingFile(filename, data, 'json');
}

export async function openTrainingExportDirectory(savedPath?: string) {
  const directory = savedPath ? await dirname(savedPath) : (await api.storageInfo()).exportDir;
  await openPath(directory);
}
