-- Drop AST cache tables

DROP INDEX IF EXISTS idx_ast_call_graph_callee;
DROP INDEX IF EXISTS idx_ast_call_graph_caller;
DROP INDEX IF EXISTS idx_ast_cross_refs_used;
DROP INDEX IF EXISTS idx_ast_cross_refs_defined;
DROP INDEX IF EXISTS idx_ast_cross_refs_symbol;
DROP INDEX IF EXISTS idx_ast_refined_file;
DROP INDEX IF EXISTS idx_ast_imports_path;
DROP INDEX IF EXISTS idx_ast_imports_file;
DROP INDEX IF EXISTS idx_ast_symbols_parent;
DROP INDEX IF EXISTS idx_ast_symbols_file;
DROP INDEX IF EXISTS idx_ast_symbols_kind;
DROP INDEX IF EXISTS idx_ast_symbols_name;
DROP INDEX IF EXISTS idx_ast_cache_updated;
DROP INDEX IF EXISTS idx_ast_cache_content_hash;
DROP INDEX IF EXISTS idx_ast_cache_language;

DROP TABLE IF EXISTS ast_call_graph;
DROP TABLE IF EXISTS ast_cross_refs;
DROP TABLE IF EXISTS ast_refined;
DROP TABLE IF EXISTS ast_imports;
DROP TABLE IF EXISTS ast_symbols;
DROP TABLE IF EXISTS ast_cache;
