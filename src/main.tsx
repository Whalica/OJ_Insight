import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import StudyAssistantWindow from './components/StudyAssistantWindow';
import './styles.css';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {new URLSearchParams(window.location.search).has('study_assistant') ? <StudyAssistantWindow /> : <App />}
  </React.StrictMode>,
);
