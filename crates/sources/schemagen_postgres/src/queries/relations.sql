-- Tables (relkind r/p) and views (relkind v/m) with comments, in the target schemas.
-- The caller splits on `kind`: 'r'/'p' become Table, 'v'/'m' become View.
-- $1 = schema names (text[])
SELECT
    n.nspname AS schema,
    c.relname AS name,
    c.relkind::text AS kind,
    obj_description(c.oid) AS comment
FROM
    pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE
    n.nspname = ANY ($1)
    AND c.relkind IN ('r', 'p', 'v', 'm');