import { initializeClientStorage } from '@/packages/client-storage';
import { createElement } from 'react';
import { ModalStorySurface, ModalStoryTheme } from '../modal-gallery/modal-story-surface';
import type { Preview } from '@storybook/react-vite';
import '../styles.css';
import './preview.css';

const preview: Preview = {
  loaders: [async () => { await initializeClientStorage(); return {}; }],
  decorators: [
    (Story, context) =>
      createElement(
        ModalStoryTheme.Provider,
        { value: context.globals.modalTheme ?? 'dark' },
        context.title.startsWith('Modals/')
          ? createElement(ModalStorySurface, { children: createElement(Story) })
          : createElement(Story)
      ),
  ],
  globalTypes: {
    modalTheme: {
      description: 'Appearance for modal gallery previews',
      toolbar: {
        icon: 'circlehollow',
        items: [
          { value: 'dark', title: 'Dark modals' },
          { value: 'light', title: 'Light modals' },
        ],
        dynamicTitle: true,
      },
    },
  },
  initialGlobals: { modalTheme: 'dark' },
  parameters: {
    backgrounds: {
      default: 'ghostex dark',
      values: [{ name: 'ghostex dark', value: '#050505' }],
    },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    layout: 'fullscreen',
  },
};

export default preview;
