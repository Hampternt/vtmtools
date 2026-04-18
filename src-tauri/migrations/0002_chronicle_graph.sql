-- Chronicles: one per running game. Deleting a chronicle cascades to its nodes and edges.
CREATE TABLE IF NOT EXISTS chronicles (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL,
    description  TEXT    NOT NULL DEFAULT '',
    created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Nodes: any discrete thing in a chronicle (area, character, institution, business, merit).
-- `type` is a freeform user-chosen string. `tags_json` is a JSON array of strings.
-- `properties_json` is a JSON array of typed Field records.
CREATE TABLE IF NOT EXISTS nodes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chronicle_id    INTEGER NOT NULL REFERENCES chronicles(id) ON DELETE CASCADE,
    type            TEXT    NOT NULL,
    label           TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    tags_json       TEXT    NOT NULL DEFAULT '[]',
    properties_json TEXT    NOT NULL DEFAULT '[]',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Edges: typed directional relationships between nodes.
-- `"contains"` is the UI's drilldown convention but the DB imposes no special meaning on it.
CREATE TABLE IF NOT EXISTS edges (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chronicle_id    INTEGER NOT NULL REFERENCES chronicles(id) ON DELETE CASCADE,
    from_node_id    INTEGER NOT NULL REFERENCES nodes(id)      ON DELETE CASCADE,
    to_node_id      INTEGER NOT NULL REFERENCES nodes(id)      ON DELETE CASCADE,
    edge_type       TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    properties_json TEXT    NOT NULL DEFAULT '[]',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),

    CHECK (from_node_id != to_node_id),
    UNIQUE (from_node_id, to_node_id, edge_type)
);

-- Indexes on common query paths.
CREATE INDEX IF NOT EXISTS idx_nodes_chronicle  ON nodes(chronicle_id);
CREATE INDEX IF NOT EXISTS idx_nodes_type       ON nodes(chronicle_id, type);
CREATE INDEX IF NOT EXISTS idx_edges_chronicle  ON edges(chronicle_id);
CREATE INDEX IF NOT EXISTS idx_edges_from       ON edges(from_node_id, edge_type);
CREATE INDEX IF NOT EXISTS idx_edges_to         ON edges(to_node_id,   edge_type);

-- Enforce "strict tree under contains": a node may have at most one contains-parent.
-- Other edge types have no such restriction.
CREATE UNIQUE INDEX IF NOT EXISTS idx_edges_contains_single_parent
    ON edges(to_node_id) WHERE edge_type = 'contains';

-- SQLite has no ON UPDATE CURRENT_TIMESTAMP, so maintain updated_at via triggers.
CREATE TRIGGER IF NOT EXISTS trg_chronicles_updated
    AFTER UPDATE ON chronicles FOR EACH ROW
    BEGIN UPDATE chronicles SET updated_at = datetime('now') WHERE id = NEW.id; END;

CREATE TRIGGER IF NOT EXISTS trg_nodes_updated
    AFTER UPDATE ON nodes FOR EACH ROW
    BEGIN UPDATE nodes SET updated_at = datetime('now') WHERE id = NEW.id; END;

CREATE TRIGGER IF NOT EXISTS trg_edges_updated
    AFTER UPDATE ON edges FOR EACH ROW
    BEGIN UPDATE edges SET updated_at = datetime('now') WHERE id = NEW.id; END;
