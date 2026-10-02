import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { applyPreferences, loadPreferences } from '../lib/preferences';
import { LocaleProvider, resolveLocale } from '../lib/i18n';
import StudyAssistant from './StudyAssistant';

export default function StudyAssistantWindow() {
  const [expanded, setExpanded] = useState(false);
  const [language, setLanguage] = useState(() => loadPreferences().language);
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () => applyPreferences(loadPreferences(), media.matches);
    apply();
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, []);
  useEffect(() => {
    const update = (event: StorageEvent) => {
      if (event.key === 'oj-insight.preferences') {
        const preferences = loadPreferences();
        setLanguage(preferences.language);
        applyPreferences(preferences, window.matchMedia('(prefers-color-scheme: dark)').matches);
      }
    };
    window.addEventListener('storage', update);
    return () => window.removeEventListener('storage', update);
  }, []);
  useEffect(() => {
    const pauseOnClose = () => {
      try {
        localStorage.setItem('oj-insight.study-assistant.visible', 'false');
      } catch { /* Local storage can be unavailable during shutdown. */ }
    };
    window.addEventListener('beforeunload', pauseOnClose);
    return () => window.removeEventListener('beforeunload', pauseOnClose);
  }, []);
  const changeExpanded = (next: boolean) => {
    void invoke('resize_study_assistant', { expanded: next }).then(() => setExpanded(next));
  };
  const close = () => {
    localStorage.setItem('oj-insight.study-assistant.visible', 'false');
    void invoke('close_study_assistant');
  };
  return <LocaleProvider locale={resolveLocale(language)}><StudyAssistant visible expanded={expanded} detached onExpand={changeExpanded} onClose={close} /></LocaleProvider>;
}
