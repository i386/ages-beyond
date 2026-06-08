# Quest Chains

## Goal

Living Quests should create follow-up quests when they complete, fail, or decay.
This turns isolated prompts into multi-era stories with continuity.

## Player Experience

A quest outcome opens a new historical problem:

- `Remember Memphis` completed by restoration creates `Crown the Restored City`.
- `Remember Memphis` completed by vengeance creates `Break Rome's Pride`.
- `Remember Memphis` failed creates `The Lost Claim`.
- `Apply Writing` completed through a project creates `Institutionalize the
  Discovery`.

The player should feel that choices steer history, not just flavor text.

## Non-Goals

- Do not build a full quest scripting language first.
- Do not let arbitrary LLM text create new quest state.
- Do not create infinite chains.

## State Model

Add chain metadata to `LivingQuest`:

- `chain_id: String`
- `chain_depth: i32`
- `parent_quest_id: Option<String>`
- `branch_key: Option<String>`
- `spawned_child_ids: Vec<String>`

The initial quest can use its own id as `chain_id`. Child quests inherit the
chain id and increment depth.

## Branch Keys

Branch keys should be deterministic and derived from the resolution:

- `completed.restore`
- `completed.vengeance`
- `completed.mercy`
- `completed.power`
- `failed.expired`
- `failed.empty_war`
- `failed.lost_claim`

Quest decision stance can refine the branch, but completion event type should
still matter.

## Chain Rules

Rust owns a table of chain templates:

- source quest kind
- branch key
- maximum depth
- child quest kind
- title template
- prompt template
- target selector

Example:

```text
source kind: restoration
branch: completed.restore
child kind: legitimacy
title: Crown {target}
prompt: {civ} restored {target}; now it must make the return accepted rule.
target: source target
max depth: 3
```

## Spawn Timing

Children should spawn immediately after the parent resolves. The observation
should include both:

- parent completion/failure projection
- child quest creation projection

This makes the handoff visible in the chronicle and quest notification feed.

## Chain Limits

To avoid runaway state:

- max depth per chain: 3 initially
- max active children per parent: 1
- max total active quests still enforced by existing cap
- no child spawn if an equivalent active quest already exists

## Prompt Context

Prompt context should include chain lineage:

- parent title
- parent resolution
- selected stance
- current chain depth

LLM naming should not decide chain structure, but it can name a chain chapter
later.

## Tests

Add Rust tests for:

- completed restoration spawns legitimacy child
- vengeance stance spawns war-pressure child
- failed quest spawns failure-memory child
- chain depth cap prevents further spawning
- snapshot persists and restores chain metadata
- duplicate child ids are not inserted twice

## Rollout

Start with three chains:

1. restoration -> legitimacy or grievance
2. war aim -> settlement or empty-war reckoning
3. breakthrough -> institution or unused-potential reckoning

Add more chains after the state model proves stable.
