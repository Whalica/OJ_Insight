import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { applyPreferences, loadPreferences } from '../lib/preferences';
import StudyAssistant from './StudyAssistant';

export default function StudyAssistantWindow() {
  const [expanded, setExpanded] = useState(false);
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () => applyPreferences(loadPreferences(), media.matches);
    apply();
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, []);
  useEffect(() => {
    const pauseOnClose = () => {
      try {
        const key = 'oj-insight.study-assistant.v1';
        const session = JSON.parse(localStorage.getItem(key) || 'null');
        if (session?.status === 'running' && typeof session.startedAt === 'number') {
          session.elapsedMs = Math.max(0, Number(session.elapsedMs) || 0) + Math.max(0, Date.now() - session.startedAt);
          session.startedAt = null;
          session.status = 'paused';
          localStorage.setItem(key, JSON.stringify(session));
        }
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
  return <StudyAssistant visible expanded={expanded} detached onExpand={changeExpanded} onClose={close} />;
}
