import { SvelteSet } from 'svelte/reactivity';

export type Stage = string;

export interface StageMachineConfig {
  tabOrder: Stage[];
  required: Stage[];
  initialConfirmed?: Stage[];
}

export interface StageMachine {
  stage: Stage;
  confirmed: Set<Stage>;
  advance(): void;
  retreat(): void;
  setStage(s: Stage): void;
  isComplete(): boolean;
}

export function createStageMachine(cfg: StageMachineConfig): StageMachine {
  const tabOrder = cfg.tabOrder;
  const required = new Set(cfg.required);
  const confirmed = new SvelteSet<Stage>(cfg.initialConfirmed ?? []);

  function firstUnconfirmed(): Stage {
    for (const s of tabOrder) if (!confirmed.has(s)) return s;
    return 'review';
  }

  let stage = $state<Stage>(
    [...required].every(r => confirmed.has(r))
      ? (cfg.initialConfirmed && cfg.initialConfirmed.length === tabOrder.length ? 'review' : firstUnconfirmed())
      : firstUnconfirmed()
  );

  if ([...tabOrder].every(s => confirmed.has(s))) stage = 'review';

  const machine: StageMachine = {
    get stage() { return stage; },
    set stage(s: Stage) { stage = s; },
    get confirmed() { return confirmed; },

    advance() {
      if (stage !== 'review') confirmed.add(stage);
      const idx = tabOrder.indexOf(stage);
      if (idx === -1 || idx >= tabOrder.length - 1) {
        stage = 'review';
      } else {
        stage = tabOrder[idx + 1];
      }
    },

    retreat() {
      const idx = tabOrder.indexOf(stage);
      if (idx > 0) {
        stage = tabOrder[idx - 1];
      } else if (stage === 'review') {
        stage = tabOrder[tabOrder.length - 1];
      }
    },

    setStage(s: Stage) {
      stage = s;
      confirmed.delete(s);
    },

    isComplete() {
      return [...required].every(r => confirmed.has(r));
    },
  };

  return machine;
}
