import { IconPlus } from '@tabler/icons-react';
import { SIDEBAR_CONTEXT_MENU_VIEWPORT_MARGIN_PX, SidebarContextMenuPortal } from '../sidebar-context-menu-portal';
import type { WebviewApi } from '../webview-api';

const ADD_PROJECT_CONTEXT_MENU_WIDTH_PX = 196;
const ADD_PROJECT_CONTEXT_MENU_HEIGHT_PX = 12 + 34;

export type SidebarAddProjectContextMenuPosition = { x: number; y: number };

/**
 * CDXC:Projects 2026-09-15 DECISION:
 * User: an empty project list (an empty Space, or no projects at all) shows an outline "Add Project" button in place of the bare "No projects" copy, and right-clicking the empty area offers the same action.
 * The button is the only entry point that does not require finding Add Project in the More menu, so it renders for every empty list that can add a project, local and remote alike.
 */
export function SidebarEmptyProjectsState({ copy, onAddProject }: { copy: string; onAddProject?: () => void }) {
  return (
    <div className='reference-sidebar-empty-state' data-sidebar-empty-projects='true'>
      {copy}
      {onAddProject ? (
        <button className='reference-sidebar-empty-state-add-project' onClick={onAddProject} type='button'>
          <IconPlus aria-hidden='true' size={14} stroke={2} />
          Add Project
        </button>
      ) : null}
    </div>
  );
}

export function clampSidebarAddProjectContextMenuPosition(
  clientX: number,
  clientY: number
): SidebarAddProjectContextMenuPosition {
  const margin = SIDEBAR_CONTEXT_MENU_VIEWPORT_MARGIN_PX;
  return {
    x: Math.max(margin, Math.min(clientX, window.innerWidth - ADD_PROJECT_CONTEXT_MENU_WIDTH_PX - margin)),
    y: Math.max(margin, Math.min(clientY, window.innerHeight - ADD_PROJECT_CONTEXT_MENU_HEIGHT_PX - margin)),
  };
}

export function SidebarAddProjectContextMenu({
  onAddProject,
  onDismiss,
  position,
  vscode,
}: {
  onAddProject: () => void;
  onDismiss: () => void;
  position: SidebarAddProjectContextMenuPosition;
  vscode?: WebviewApi;
}) {
  return (
    <SidebarContextMenuPortal
      menuStyle={{
        left: `${position.x}px`,
        top: `${position.y}px`,
        width: `${ADD_PROJECT_CONTEXT_MENU_WIDTH_PX}px`,
      }}
      onDismiss={onDismiss}
      vscode={vscode}
    >
      <button
        className='session-context-menu-item'
        onClick={() => {
          onDismiss();
          onAddProject();
        }}
        role='menuitem'
        type='button'
      >
        <IconPlus aria-hidden='true' className='session-context-menu-icon' size={14} stroke={2} />
        Add Project
      </button>
    </SidebarContextMenuPortal>
  );
}
