import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { App } from './App';
import { ToastContainer } from '@/presentation/components/Toast';
import './index.css';

createRoot(document.getElementById('root')!).render(
  <BrowserRouter>
    <App />
    <ToastContainer />
  </BrowserRouter>
);
