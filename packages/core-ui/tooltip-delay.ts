import { TooltipProvider } from '../components/ui/tooltip';
import { createContext, createElement, useContext, type ReactNode } from 'react';

/** The Settings default for `sidebarTooltipDelayMs` (packages/settings-catalog/src/data/defaults.rs). */
const DEFAULT_SIDEBAR_TOOLTIP_DELAY_MS = 600;

const SidebarTooltipDelayContext = createContext<number | undefined>(undefined);

/**
 * CDXC:Tooltips 2026-09-16 DECISION:
 * User: all sidebar elements, including project titles, must follow the tooltip delay in Settings.
 * Use the exact configured delay without per-control offsets; AppTooltip isolates each hover delay so moving between labels cannot skip it.
 */
export function SidebarTooltipDelayProvider({ children, delayMs }: { children: ReactNode; delayMs: number }) {
  return createElement(
    SidebarTooltipDelayContext.Provider,
    { value: delayMs },
    createElement(TooltipProvider, { delayDuration: delayMs }, children)
  );
}

export function useConfiguredSidebarTooltipDelayMs(): number | undefined {
  return useContext(SidebarTooltipDelayContext);
}

export function useSidebarTooltipDelayMs(): number {
  return useConfiguredSidebarTooltipDelayMs() ?? DEFAULT_SIDEBAR_TOOLTIP_DELAY_MS;
}
