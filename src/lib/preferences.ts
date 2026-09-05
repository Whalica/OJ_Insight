export type ThemeMode = 'system' | 'light' | 'dark';
export type FontSize = 'standard' | 'large' | 'xlarge';
export type InterfaceDensity = 'comfortable' | 'compact';
export type HeatmapPalette = 'green' | 'blue' | 'accessible';
export type StartupPage = 'overview' | 'last';

export interface Preferences {
  theme: ThemeMode;
  fontSize: FontSize;
  density: InterfaceDensity;
  heatmapPalette: HeatmapPalette;
  reduceMotion: boolean;
  startupPage: StartupPage;
}

export const DEFAULT_PREFERENCES: Preferences = {
  theme: 'system',
  fontSize: 'standard',
  density: 'comfortable',
  heatmapPalette: 'green',
  reduceMotion: false,
  startupPage: 'overview',
};

const STORAGE_KEY = 'oj-insight.preferences';

function member<T extends string>(value: unknown, choices: readonly T[], fallback: T): T {
  return typeof value === 'string' && choices.includes(value as T) ? value as T : fallback;
}

export function loadPreferences(): Preferences {
  try {
    const value = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}') as Partial<Preferences>;
    return {
      theme: member(value.theme, ['system', 'light', 'dark'], 'system'),
      fontSize: member(value.fontSize, ['standard', 'large', 'xlarge'], 'standard'),
      density: member(value.density, ['comfortable', 'compact'], 'comfortable'),
      heatmapPalette: member(value.heatmapPalette, ['green', 'blue', 'accessible'], 'green'),
      reduceMotion: value.reduceMotion === true,
      startupPage: member(value.startupPage, ['overview', 'last'], 'overview'),
    };
  } catch {
    return { ...DEFAULT_PREFERENCES };
  }
}

export function savePreferences(value: Preferences) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
}

export function applyPreferences(value: Preferences, systemDark: boolean) {
  const root = document.documentElement;
  root.dataset.theme = value.theme === 'system' ? (systemDark ? 'dark' : 'light') : value.theme;
  root.dataset.fontSize = value.fontSize;
  root.dataset.density = value.density;
  root.dataset.heatmapPalette = value.heatmapPalette;
  root.dataset.reduceMotion = String(value.reduceMotion);
  root.style.colorScheme = root.dataset.theme;
}
