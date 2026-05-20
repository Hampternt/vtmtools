-- Item Builder Shell: support saving a draft (in-progress) row even when its
-- required fields are incomplete, plus a stage-completion checklist used by
-- the builder's reopen flow.
--
-- wip              : 0 = complete row (normal save), 1 = draft (validation
--                    bypass; surfaced in lists with a "draft" affordance).
-- confirmed_fields : JSON array of stage keys the user explicitly confirmed.
--                    Disambiguates "confirmed empty" from "never reached"
--                    when reopening a draft in the multi-stage builder.
--                    NULL on legacy rows (treated as "all stages confirmed"
--                    since they predate the builder).
ALTER TABLE advantages       ADD COLUMN wip              INTEGER NOT NULL DEFAULT 0;
ALTER TABLE advantages       ADD COLUMN confirmed_fields TEXT;
ALTER TABLE status_templates ADD COLUMN wip              INTEGER NOT NULL DEFAULT 0;
ALTER TABLE status_templates ADD COLUMN confirmed_fields TEXT;
