import { save } from '@tauri-apps/plugin-dialog';
import { sep } from '@tauri-apps/api/path';
import { api } from './api';

export async function saveTrainingJson(filename: string, data: string) {
  const storage = await api.storageInfo();
  const separator = sep();
  const slash = storage.exportDir.endsWith('/') || storage.exportDir.endsWith('\\') ? '' : separator;
  const path = await save({ defaultPath: `${storage.exportDir}${slash}${filename}`, filters: [{ name: 'JSON', extensions: ['json'] }] });
  if (!path) return false;
  await api.writeExportFile(path, Array.from(new TextEncoder().encode(data)));
  return true;
}
