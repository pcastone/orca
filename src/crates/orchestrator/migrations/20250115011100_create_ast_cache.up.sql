-- Create AST cache tables for semantic code indexing
-- Stores parsed AST data and refined semantic information

-- Main AST cache table for parsed files
CREATE TABLE IF NOT EXISTS ast_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL UNIQUE,
    language TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    symbols_json TEXT NOT NULL,
    imports_json TEXT NOT NULL,
    ast_json TEXT,
    parse_duration_ms INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Symbols table for fast lookups
CREATE TABLE IF NOT EXISTS ast_symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    visibility TEXT,
    line INTEGER NOT NULL,
    column INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    end_column INTEGER NOT NULL,
    parent TEXT,
    documentation TEXT,
    return_type TEXT,
    parameters_json TEXT,
    FOREIGN KEY (file_id) REFERENCES ast_cache(id) ON DELETE CASCADE
);

-- Imports table for dependency tracking
CREATE TABLE IF NOT EXISTS ast_imports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    names_json TEXT NOT NULL,
    alias TEXT,
    line INTEGER NOT NULL,
    is_relative INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (file_id) REFERENCES ast_cache(id) ON DELETE CASCADE
);

-- Refined AST data for deeper semantic analysis
CREATE TABLE IF NOT EXISTS ast_refined (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL UNIQUE,
    call_graph_json TEXT NOT NULL,
    type_info_json TEXT NOT NULL,
    cross_refs_json TEXT NOT NULL,
    refinement_level INTEGER NOT NULL DEFAULT 1,
    refined_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (file_id) REFERENCES ast_cache(id) ON DELETE CASCADE
);

-- Cross-references table for tracking symbol usage
CREATE TABLE IF NOT EXISTS ast_cross_refs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_name TEXT NOT NULL,
    defined_file_id INTEGER NOT NULL,
    used_file_id INTEGER NOT NULL,
    used_line INTEGER NOT NULL,
    used_column INTEGER NOT NULL,
    reference_kind TEXT NOT NULL,
    context TEXT,
    FOREIGN KEY (defined_file_id) REFERENCES ast_cache(id) ON DELETE CASCADE,
    FOREIGN KEY (used_file_id) REFERENCES ast_cache(id) ON DELETE CASCADE
);

-- Call graph edges for function call tracking
CREATE TABLE IF NOT EXISTS ast_call_graph (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    caller_file_id INTEGER NOT NULL,
    caller_function TEXT NOT NULL,
    callee TEXT NOT NULL,
    callee_file_id INTEGER,
    line INTEGER NOT NULL,
    arguments_json TEXT,
    FOREIGN KEY (caller_file_id) REFERENCES ast_cache(id) ON DELETE CASCADE,
    FOREIGN KEY (callee_file_id) REFERENCES ast_cache(id) ON DELETE SET NULL
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_ast_cache_language ON ast_cache(language);
CREATE INDEX IF NOT EXISTS idx_ast_cache_content_hash ON ast_cache(content_hash);
CREATE INDEX IF NOT EXISTS idx_ast_cache_updated ON ast_cache(updated_at);

CREATE INDEX IF NOT EXISTS idx_ast_symbols_name ON ast_symbols(name);
CREATE INDEX IF NOT EXISTS idx_ast_symbols_kind ON ast_symbols(kind);
CREATE INDEX IF NOT EXISTS idx_ast_symbols_file ON ast_symbols(file_id);
CREATE INDEX IF NOT EXISTS idx_ast_symbols_parent ON ast_symbols(parent);

CREATE INDEX IF NOT EXISTS idx_ast_imports_file ON ast_imports(file_id);
CREATE INDEX IF NOT EXISTS idx_ast_imports_path ON ast_imports(path);

CREATE INDEX IF NOT EXISTS idx_ast_refined_file ON ast_refined(file_id);

CREATE INDEX IF NOT EXISTS idx_ast_cross_refs_symbol ON ast_cross_refs(symbol_name);
CREATE INDEX IF NOT EXISTS idx_ast_cross_refs_defined ON ast_cross_refs(defined_file_id);
CREATE INDEX IF NOT EXISTS idx_ast_cross_refs_used ON ast_cross_refs(used_file_id);

CREATE INDEX IF NOT EXISTS idx_ast_call_graph_caller ON ast_call_graph(caller_file_id, caller_function);
CREATE INDEX IF NOT EXISTS idx_ast_call_graph_callee ON ast_call_graph(callee);
