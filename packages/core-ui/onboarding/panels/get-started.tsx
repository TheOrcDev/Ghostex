import { useState } from 'react';
import type { PreferredAgentInterface } from '@/packages/shared/ghostex-settings';
import {
  defaultAgentId as resolveDefaultAgentId,
  finishOnboarding,
  installedAgents,
  type PanelProps,
} from '../onboarding-state';
import { Cta, Eyebrow, FootActions, Heading, Icon, Spinner, Sub } from '../primitives';
import { box } from '../stage';

const SESSION_VIEWS: readonly (readonly [PreferredAgentInterface, string, string])[] = [
  ['chat', 'Chat', 'Cleaner agent conversation'],
  ['terminal', 'Terminal', 'Raw CLI interface'],
];
const CARD_LEFT = 486;
const CARD_TOP = 310;
const CARD_WIDTH = 700;
/**
 * CDXC:Onboarding 2026-09-15 WHY:
 * The prototype laid the "Start with" tiles out at a fixed pitch of card width over tile count, which squeezed
 * fifteen installed agent CLIs into 36px tiles with every name overlapping. The card is flow layout now: up to
 * this many choices keep the name-and-detail tiles in one row, more become name-only chips that wrap, and the
 * card grows with them.
 */
const MAX_TILE_ROW = 4;

export function GetStartedPanel({ props, flow, setFlow }: PanelProps) {
  const { settings, agents, pickedProjectFolder, hasProjects } = props;
  const defaultAgent = resolveDefaultAgentId(settings, agents);
  const installed = installedAgents(agents);
  const ordered = defaultAgent
    ? [
        installed.find((agent) => agent.agentId === defaultAgent)!,
        ...installed.filter((agent) => agent.agentId !== defaultAgent),
      ]
    : installed;
  const tiles: readonly { id: string; name: string; detail: string }[] = [
    ...ordered.map((agent) => ({
      id: agent.agentId,
      name: agent.name,
      detail: agent.agentId === defaultAgent ? 'Default agent' : 'Switch anytime',
    })),
    { id: 'terminal', name: 'Terminal', detail: 'No agent yet' },
  ];
  const startWith = flow.startWith ?? defaultAgent ?? 'terminal';
  const sessionView = settings?.preferredAgentInterface ?? 'chat';
  const folder = pickedProjectFolder?.trim() ?? '';
  const canOpen = folder.length > 0 || hasProjects === true;
  /** "Open Ghostex" is a host round trip: busy until the project and session exist, error stays on the panel. */
  const [opening, setOpening] = useState(false);
  const [openError, setOpenError] = useState<string>();

  const setSessionView = (view: PreferredAgentInterface) => {
    if (!settings) return;
    props.onChange({ ...settings, preferredAgentInterface: view });
  };
  const openGhostex = () => {
    if (folder) {
      if (opening) return;
      setOpening(true);
      setOpenError(undefined);
      props.onFinishFirstLaunch({ agentId: startWith, path: folder }).then(
        () => setFlow({ finishedPath: folder, finished: true }),
        (error: unknown) => {
          setOpening(false);
          setOpenError(error instanceof Error && error.message ? error.message : 'Ghostex could not open the project.');
        }
      );
      return;
    }
    if (hasProjects) finishOnboarding(props, flow);
  };
  const advancedLater = () => {
    props.onOpenSettings?.();
    props.onClose();
  };

  return (
    <>
      <Eyebrow x={486} y={150} w={700}>
        Get started
      </Eyebrow>
      <Heading x={336} y={182} w={1000} size={48} center l1='Open your first project in Ghostex.' />
      <Sub x={486} y={252} w={700} size={16.5} center>
        One folder, one agent, one default view. Everything else can change later.
      </Sub>
      <div className='pcol' style={box(CARD_LEFT, CARD_TOP, CARD_WIDTH)}>
        <div className='glass pcard'>
          <div className='label'>Project folder</div>
          <div className='pfield'>
            <Icon n='folder' size={22} className='dimc2' />
            <span className={'path' + (folder ? '' : ' dim')}>{folder || 'Choose a folder to start in'}</span>
            <button type='button' className='choose' onClick={props.onPickProjectFolder}>
              Choose folder
            </button>
          </div>
          <div className='label'>Start with</div>
          {tiles.length <= MAX_TILE_ROW ? (
            <div className='opts' style={{ gridTemplateColumns: `repeat(${tiles.length}, minmax(0, 1fr))` }}>
              {tiles.map((tile) => (
                <button
                  key={tile.id}
                  type='button'
                  className={'opt' + (startWith === tile.id ? ' sel' : '')}
                  onClick={() => setFlow({ startWith: tile.id })}
                >
                  <span className='nm'>{tile.name}</span>
                  <span className='ss'>{tile.detail}</span>
                </button>
              ))}
            </div>
          ) : (
            <div className='chips'>
              {tiles.map((tile) => (
                <button
                  key={tile.id}
                  type='button'
                  className={'opt chip' + (startWith === tile.id ? ' sel' : '')}
                  title={tile.detail}
                  onClick={() => setFlow({ startWith: tile.id })}
                >
                  {tile.name}
                </button>
              ))}
            </div>
          )}
          <div className='label'>Default session view</div>
          <div className='opts' style={{ gridTemplateColumns: `repeat(${SESSION_VIEWS.length}, minmax(0, 1fr))` }}>
            {SESSION_VIEWS.map(([id, name, detail]) => (
              <button
                key={id}
                type='button'
                className={'opt' + (sessionView === id ? ' sel' : '')}
                onClick={() => setSessionView(id)}
              >
                <span className='nm'>{name}</span>
                <span className='ss'>{detail}</span>
              </button>
            ))}
          </div>
        </div>
        <p className='sub center pnote-flow'>
          {openError ? (
            <span style={{ color: '#ff6b62' }} role='alert'>
              {openError}
            </span>
          ) : opening ? (
            'Adding the project and opening its first session…'
          ) : canOpen ? (
            "That's it. The workspace teaches the deeper features once you are inside."
          ) : (
            'Choose a folder above to open your first project.'
          )}
        </p>
      </div>
      <FootActions panel={5}>
        <button type='button' className='ghost' onClick={advancedLater} disabled={opening}>
          Advanced settings later
        </button>
        <Cta filled onClick={openGhostex} disabled={!canOpen || opening} arrow={!opening}>
          {opening ? (
            <>
              <Spinner /> Opening…
            </>
          ) : (
            'Open Ghostex'
          )}
        </Cta>
      </FootActions>
    </>
  );
}
