// vtmtools — Foundry roll-result hook.
// Subscribes to createChatMessage; serializes the roll into the wire
// shape vtmtools' Rust translator expects, and ships it on the open socket.
//
// Splat detection: regex on roll.formula per docs/reference/foundry-vtm5e-rolls.md
// §"Splat detection" — roll.system is unreliable post-rehydration.
//
// Per-die-class extraction: walk roll.terms[] for Die instances, partition by
// denomination (basic: m/v/w/h, advanced: g/r/s).

// JS regex caveat: digits are word characters, so `\b` between a digit and
// `d` never fires — real formulas always look like `12dv...`, which is
// word→word at the digit→`d` boundary. Drop the leading `\b` and use
// `(?!\w)` after the denomination to keep hypothetical multi-char denoms
// (e.g. `dvs`) from false-matching `dv`.
const SPLAT_PATTERNS = {
  vampire: /d[vg](?!\w)/,
  werewolf: /d[wr](?!\w)/,
  hunter: /d[hs](?!\w)/,
  mortal: /dm(?!\w)/,
};

function detectSplat(formula) {
  if (!formula) return 'unknown';
  if (SPLAT_PATTERNS.vampire.test(formula)) return 'vampire';
  if (SPLAT_PATTERNS.werewolf.test(formula)) return 'werewolf';
  if (SPLAT_PATTERNS.hunter.test(formula)) return 'hunter';
  if (SPLAT_PATTERNS.mortal.test(formula)) return 'mortal';
  return 'unknown';
}

const BASIC_DENOMS = new Set(['m', 'v', 'w', 'h']);
const ADV_DENOMS = new Set(['g', 'r', 's']);

function extractDiceResults(roll, kind) {
  const out = [];
  const want = kind === 'basic' ? BASIC_DENOMS : ADV_DENOMS;
  const terms = Array.isArray(roll?.terms) ? roll.terms : [];
  for (const term of terms) {
    // Term may be a Die instance, a NumericTerm, or an OperatorTerm. Filter to dice.
    const isDie = term?.constructor?.name === 'Die' || term?.class === 'Die';
    if (!isDie) continue;
    const denom = String(term.denomination ?? '').toLowerCase();
    if (!want.has(denom)) continue;
    for (const r of term.results ?? []) {
      if (typeof r?.result === 'number') out.push(r.result);
    }
  }
  return out;
}

function messageToWire(message) {
  const roll = message.rolls?.[0];
  if (!roll) return null;

  const formula = roll.formula ?? '';
  const splat = detectSplat(formula);

  const ts = typeof message.timestamp === 'number'
    ? new Date(message.timestamp).toISOString()
    : (typeof message.timestamp === 'string' ? message.timestamp : null);

  let raw;
  try { raw = message.toJSON ? message.toJSON() : message; }
  catch { raw = {}; }

  return {
    message_id: message._id ?? message.id ?? '',
    actor_id: message.speaker?.actor ?? null,
    actor_name: message.speaker?.alias ?? null,
    timestamp: ts,
    flavor: typeof message.flavor === 'string' ? message.flavor : '',
    formula,
    splat,
    basic_results: extractDiceResults(roll, 'basic'),
    advanced_results: extractDiceResults(roll, 'advanced'),
    total: typeof roll.total === 'number' ? roll.total : 0,
    difficulty: typeof roll.options?.difficulty === 'number' ? roll.options.difficulty : null,
    raw,
  };
}

export function init(getSocket) {
  Hooks.on('createChatMessage', (message, _options, _userId) => {
    if (!message.rolls?.length) return;
    const socket = getSocket();
    if (!socket || socket.readyState !== WebSocket.OPEN) return;
    const wire = messageToWire(message);
    if (!wire) return;
    try {
      socket.send(JSON.stringify({ type: 'roll_result', message: wire }));
    } catch (err) {
      console.error('[vtmtools-bridge] roll-result send failed:', err);
    }
  });
}
