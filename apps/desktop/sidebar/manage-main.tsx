import { initializeClientStorage } from '@/packages/client-storage';
import { installManageCefBridge } from './project-workarea-cef-bridge';
import { installWorkareaTheme } from '../views/workarea-theme';
import '@/packages/core-ui/styles/shadcn.generated.css';
import '@/packages/core-ui/styles/theme.css';

installManageCefBridge();
installWorkareaTheme();

void initializeClientStorage()
  .then(() => import('../views/manage'))
  .catch((error) => {
    const root = document.getElementById('root');
    if (root) root.textContent = `Could not load Files: ${error instanceof Error ? error.message : String(error)}`;
  });
