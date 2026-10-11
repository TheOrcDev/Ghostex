import { retainAppScrollbars } from '@/packages/components/ui/app-scrollbars';
import '@fontsource-variable/inter';
import { createRoot } from 'react-dom/client';
import { ManageEmbed } from './manage/embed';
import { MANAGE_STYLES } from './manage/styles';

const styleElement = document.createElement('style');
styleElement.textContent = MANAGE_STYLES;
document.head.append(styleElement);

createRoot(document.getElementById('root')!).render(<ManageEmbed />);

const releaseScrollbars = retainAppScrollbars();
window.addEventListener('pagehide', releaseScrollbars, { once: true });
